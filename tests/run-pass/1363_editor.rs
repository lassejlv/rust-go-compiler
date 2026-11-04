// A line-based text editor core: gap-free line buffer, cursors, selections,
// an undo/redo stack of inverse operations, search/replace, macros recorded as
// closures, and a tree of folds. Written with many NLL-dependent borrow
// patterns (two-phase borrows, disjoint field borrows in closures, reborrows
// in loops, split_at_mut, mem::take-and-restore).
use std::collections::{BTreeMap, HashMap};
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Default)]
struct Pos {
    line: usize,
    col: usize,
}

#[derive(Debug, Clone, PartialEq)]
enum Edit {
    Insert { at: Pos, text: String },
    Delete { from: Pos, to: Pos, removed: String },
}

impl Edit {
    fn inverse(&self) -> Edit {
        match self {
            Edit::Insert { at, text } => {
                let to = advance(*at, text);
                Edit::Delete { from: *at, to, removed: text.clone() }
            }
            Edit::Delete { from, removed, .. } => Edit::Insert { at: *from, text: removed.clone() },
        }
    }
}

fn advance(mut p: Pos, text: &str) -> Pos {
    for c in text.chars() {
        if c == '\n' {
            p.line += 1;
            p.col = 0;
        } else {
            p.col += 1;
        }
    }
    p
}

#[derive(Default)]
struct Buffer {
    lines: Vec<String>,
    cursor: Pos,
    undo: Vec<Vec<Edit>>,
    redo: Vec<Vec<Edit>>,
    group: Option<Vec<Edit>>,
    marks: BTreeMap<char, Pos>,
    stats: HashMap<&'static str, usize>,
}

impl fmt::Display for Buffer {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        for (i, l) in self.lines.iter().enumerate() {
            let marker = if i == self.cursor.line { '>' } else { ' ' };
            writeln!(f, "{}{:>3}| {}", marker, i + 1, l)?;
        }
        Ok(())
    }
}

fn byte_idx(s: &str, col: usize) -> usize {
    s.char_indices().nth(col).map_or(s.len(), |(i, _)| i)
}

impl Buffer {
    fn from_text(t: &str) -> Self {
        let mut b = Buffer { lines: t.split('\n').map(String::from).collect(), ..Default::default() };
        if b.lines.is_empty() {
            b.lines.push(String::new());
        }
        b
    }
    fn text(&self) -> String {
        self.lines.join("\n")
    }
    fn bump(&mut self, k: &'static str) {
        // two-phase borrow: the map is borrowed mutably while computing the default
        let n = self.stats.len();
        *self.stats.entry(k).or_insert(n * 0) += 1;
    }
    fn clamp(&self, p: Pos) -> Pos {
        let line = p.line.min(self.lines.len() - 1);
        Pos { line, col: p.col.min(self.lines[line].chars().count()) }
    }
    fn raw_insert(&mut self, at: Pos, text: &str) -> Pos {
        let at = self.clamp(at);
        let line = &mut self.lines[at.line];
        let bi = byte_idx(line, at.col);
        let tail = line.split_off(bi);
        let mut parts = text.split('\n');
        line.push_str(parts.next().unwrap());
        let mut cur = at.line;
        for p in parts {
            cur += 1;
            self.lines.insert(cur, p.to_string());
        }
        let end = advance(at, text);
        self.lines[cur].push_str(&tail);
        end
    }
    fn raw_delete(&mut self, from: Pos, to: Pos) -> String {
        let (from, to) = (self.clamp(from.min(to)), self.clamp(from.max(to)));
        if from.line == to.line {
            let l = &mut self.lines[from.line];
            let (a, b) = (byte_idx(l, from.col), byte_idx(l, to.col));
            return l.drain(a..b).collect();
        }
        let mut removed = String::new();
        let first_tail = {
            let l = &mut self.lines[from.line];
            let a = byte_idx(l, from.col);
            l.split_off(a)
        };
        removed.push_str(&first_tail);
        let middle: Vec<String> = self.lines.drain(from.line + 1..to.line).collect();
        for m in &middle {
            removed.push('\n');
            removed.push_str(m);
        }
        let last = self.lines.remove(from.line + 1);
        let b = byte_idx(&last, to.col);
        removed.push('\n');
        removed.push_str(&last[..b]);
        self.lines[from.line].push_str(&last[b..]);
        removed
    }
    fn apply(&mut self, e: &Edit) {
        match e {
            Edit::Insert { at, text } => {
                self.cursor = self.raw_insert(*at, text);
            }
            Edit::Delete { from, to, .. } => {
                self.raw_delete(*from, *to);
                self.cursor = *from;
            }
        }
    }
    fn record(&mut self, e: Edit) {
        self.apply(&e);
        self.redo.clear();
        match self.group.as_mut() {
            Some(g) => g.push(e),
            None => self.undo.push(vec![e]),
        }
    }
    fn insert(&mut self, text: &str) {
        self.bump("insert");
        let at = self.cursor;
        self.record(Edit::Insert { at, text: text.to_string() });
    }
    fn delete_back(&mut self, n: usize) {
        self.bump("delete");
        let to = self.cursor;
        let mut from = to;
        for _ in 0..n {
            if from.col > 0 {
                from.col -= 1;
            } else if from.line > 0 {
                from.line -= 1;
                from.col = self.lines[from.line].chars().count();
            }
        }
        let removed = {
            let mut probe = Buffer { lines: self.lines.clone(), ..Default::default() };
            probe.raw_delete(from, to)
        };
        self.record(Edit::Delete { from, to, removed });
    }
    fn begin_group(&mut self) {
        self.group = Some(Vec::new());
    }
    fn end_group(&mut self) {
        if let Some(g) = self.group.take() {
            if !g.is_empty() {
                self.undo.push(g);
            }
        }
    }
    fn undo(&mut self) -> bool {
        let Some(group) = self.undo.pop() else { return false };
        for e in group.iter().rev() {
            self.apply(&e.inverse());
        }
        self.redo.push(group);
        true
    }
    fn redo(&mut self) -> bool {
        match self.redo.pop() {
            Some(group) => {
                for e in &group {
                    self.apply(e);
                }
                self.undo.push(group);
                true
            }
            None => false,
        }
    }
    fn find_all(&self, needle: &str) -> Vec<Pos> {
        self.lines
            .iter()
            .enumerate()
            .flat_map(|(i, l)| l.match_indices(needle).map(move |(b, _)| Pos { line: i, col: l[..b].chars().count() }))
            .collect()
    }
    fn replace_all(&mut self, needle: &str, with: &str) -> usize {
        let hits = self.find_all(needle);
        self.begin_group();
        // apply from the end so earlier positions stay valid
        for p in hits.iter().rev() {
            let to = Pos { line: p.line, col: p.col + needle.chars().count() };
            self.record(Edit::Delete { from: *p, to, removed: needle.to_string() });
            self.record(Edit::Insert { at: *p, text: with.to_string() });
        }
        self.end_group();
        hits.len()
    }
    fn indent_lines(&mut self, range: std::ops::Range<usize>, width: usize) {
        // disjoint borrows: closure reads `self.stats`'s sibling field while lines are mutated
        let pad = " ".repeat(width);
        let marks = &self.marks;
        let mut touched = 0;
        for (i, line) in self.lines.iter_mut().enumerate().filter(|(i, _)| range.contains(i)) {
            if !line.is_empty() && !marks.values().any(|m| m.line == i && m.col == usize::MAX) {
                line.insert_str(0, &pad);
                touched += 1;
            }
        }
        *self.stats.entry("indent").or_default() += touched;
    }
    fn swap_lines(&mut self, a: usize, b: usize) {
        if a == b {
            return;
        }
        let (lo, hi) = (a.min(b), a.max(b));
        let (left, right) = self.lines.split_at_mut(hi);
        std::mem::swap(&mut left[lo], &mut right[0]);
    }
    fn longest_line_mut(&mut self) -> Option<&mut String> {
        let mut best: Option<&mut String> = None;
        for l in self.lines.iter_mut() {
            match &best {
                Some(b) if b.len() >= l.len() => {}
                _ => best = Some(l),
            }
        }
        best
    }
    fn sort_paragraph(&mut self) {
        let mut lines = std::mem::take(&mut self.lines);
        let n = lines.len();
        // find the paragraph containing the cursor and sort it, then restore
        let start = lines[..=self.cursor.line.min(n - 1)].iter().rposition(|l| l.trim().is_empty()).map_or(0, |i| i + 1);
        let end = lines[start..].iter().position(|l| l.trim().is_empty()).map_or(n, |i| start + i);
        lines[start..end].sort_by_key(|l| l.to_lowercase());
        self.lines = lines;
        self.bump("sort");
    }
}

// folding tree with nested ranges
#[derive(Debug)]
struct Fold {
    start: usize,
    end: usize,
    children: Vec<Fold>,
}

fn build_folds(lines: &[String]) -> Vec<Fold> {
    let indent = |l: &String| if l.trim().is_empty() { usize::MAX } else { l.len() - l.trim_start().len() };
    fn go(lines: &[String], lo: usize, hi: usize, indent: &dyn Fn(&String) -> usize) -> Vec<Fold> {
        let mut out = vec![];
        let mut i = lo;
        while i < hi {
            let base = indent(&lines[i]);
            let mut j = i + 1;
            while j < hi && (indent(&lines[j]) > base) {
                j += 1;
            }
            if j > i + 1 {
                out.push(Fold { start: i, end: j, children: go(lines, i + 1, j, indent) });
            }
            i = j;
        }
        out
    }
    go(lines, 0, lines.len(), &indent)
}

fn show_folds(fs: &[Fold], depth: usize, out: &mut Vec<String>) {
    for f in fs {
        out.push(format!("{}{}..{}", "  ".repeat(depth), f.start + 1, f.end));
        show_folds(&f.children, depth + 1, out);
    }
}

fn main() {
    let mut b = Buffer::from_text("fn main() {\n    let x = 1;\n    let y = 2;\n}\n\nzeta\nalpha\nMike\nbravo");
    print!("{}", b);
    b.cursor = Pos { line: 1, col: 14 };
    b.insert(" // one");
    b.cursor = Pos { line: 3, col: 0 };
    b.insert("    println!(\"{}\", x + y);\n");
    print!("--- after inserts\n{}", b);
    b.delete_back(3);
    print!("--- after delete_back(3) cursor {:?}\n{}", b.cursor, b);
    b.cursor = Pos { line: 4, col: 1 };
    b.delete_back(2);
    print!("--- join lines, cursor {:?}\n{}", b.cursor, b);
    let n = b.replace_all("let", "const");
    print!("--- replaced {}\n{}", n, b);
    println!("find x: {:?}", b.find_all("x"));
    let mut undone = 0;
    while b.undo() {
        undone += 1;
    }
    println!("undid {} groups; text restored: {}", undone, b.text() == "fn main() {\n    let x = 1;\n    let y = 2;\n}\n\nzeta\nalpha\nMike\nbravo");
    b.redo();
    b.redo();
    print!("--- after 2 redos\n{}", b);
    b.cursor = Pos { line: 7, col: 0 };
    b.sort_paragraph();
    b.indent_lines(6..9, 2);
    b.swap_lines(0, 4);
    b.swap_lines(4, 0);
    if let Some(l) = b.longest_line_mut() {
        l.push_str(" // longest");
    }
    print!("--- sorted/indented\n{}", b);
    b.marks.insert('a', Pos { line: 2, col: 3 });
    b.marks.insert('b', b.cursor);
    println!("marks {:?}", b.marks);
    let mut stats: Vec<_> = b.stats.iter().map(|(k, v)| (*k, *v)).collect();
    stats.sort();
    println!("stats {:?} undo={} redo={}", stats, b.undo.len(), b.redo.len());

    // a macro is a list of boxed closures run against the buffer
    let mut macro_steps: Vec<Box<dyn Fn(&mut Buffer)>> = Vec::new();
    macro_steps.push(Box::new(|b| b.cursor = Pos { line: b.cursor.line, col: 0 }));
    macro_steps.push(Box::new(|b| b.insert("# ")));
    macro_steps.push(Box::new(|b| b.cursor.line += 1));
    let mut m = Buffer::from_text("one\ntwo\nthree\nfour");
    for _ in 0..3 {
        for step in &macro_steps {
            step(&mut m);
        }
    }
    print!("--- macro\n{}", m);

    let code = "a\n  b\n    c\n    d\n  e\nf\n  g\n\n  h";
    let lines: Vec<String> = code.lines().map(String::from).collect();
    let folds = build_folds(&lines);
    let mut out = vec![];
    show_folds(&folds, 0, &mut out);
    println!("folds: {:?}", out);

    // two-phase borrows and index-based mutation
    let mut v = vec![1usize, 2, 3];
    v.push(v.len());
    v[0] = v[1] + v[2];
    let last = v.len() - 1;
    v[last] *= v[0];
    let mut counts: HashMap<usize, usize> = HashMap::new();
    for i in 0..v.len() {
        let k = v[i] % 3;
        counts.insert(k, counts.get(&k).copied().unwrap_or(0) + 1);
    }
    let mut cv: Vec<_> = counts.into_iter().collect();
    cv.sort();
    println!("{:?} {:?}", v, cv);
    let mut grid = vec![vec![0u8; 4]; 3];
    for (r, row) in grid.iter_mut().enumerate() {
        for (c, cell) in row.iter_mut().enumerate() {
            *cell = (r * 4 + c) as u8;
        }
    }
    let (top, rest) = grid.split_first_mut().unwrap();
    for row in rest.iter_mut() {
        for (a, b) in row.iter_mut().zip(top.iter()) {
            *a += *b;
        }
    }
    println!("{:?}", grid);
}
