// Text editor buffer with a command pattern: Box<dyn Command> with apply/undo,
// undo/redo stacks, command merging (consecutive inserts), macros as
// composite commands, cursor movement by chars (UTF-8 aware) and a history log.
use std::fmt;

#[derive(Debug, Clone, Default)]
struct Buffer { text: String, cursor: usize } // cursor is a byte index on a char boundary

impl Buffer {
    fn prev_boundary(&self, i: usize) -> usize { self.text[..i].char_indices().next_back().map_or(0, |(j, _)| j) }
    fn next_boundary(&self, i: usize) -> usize { self.text[i..].chars().next().map_or(i, |c| i + c.len_utf8()) }
}

trait Command: fmt::Debug {
    fn apply(&mut self, b: &mut Buffer);
    fn undo(&mut self, b: &mut Buffer);
    fn merge(&mut self, _other: &dyn Command) -> bool { false }
    fn as_insert(&self) -> Option<(usize, &str)> { None }
    fn name(&self) -> String;
}

#[derive(Debug)]
struct Insert { at: usize, s: String }
impl Command for Insert {
    fn apply(&mut self, b: &mut Buffer) { b.text.insert_str(self.at, &self.s); b.cursor = self.at + self.s.len(); }
    fn undo(&mut self, b: &mut Buffer) { b.text.replace_range(self.at..self.at + self.s.len(), ""); b.cursor = self.at; }
    fn merge(&mut self, other: &dyn Command) -> bool {
        match other.as_insert() {
            Some((at, s)) if at == self.at + self.s.len() && !s.contains(' ') && !self.s.ends_with(' ') => { self.s.push_str(s); true }
            _ => false,
        }
    }
    fn as_insert(&self) -> Option<(usize, &str)> { Some((self.at, &self.s)) }
    fn name(&self) -> String { format!("insert {:?}@{}", self.s, self.at) }
}

#[derive(Debug)]
struct Delete { at: usize, len: usize, removed: Option<String> }
impl Command for Delete {
    fn apply(&mut self, b: &mut Buffer) {
        let end = (self.at + self.len).min(b.text.len());
        self.removed = Some(b.text.drain(self.at..end).collect());
        b.cursor = self.at;
    }
    fn undo(&mut self, b: &mut Buffer) {
        if let Some(r) = self.removed.take() { b.text.insert_str(self.at, &r); b.cursor = self.at + r.len(); }
    }
    fn name(&self) -> String { format!("delete {}@{}", self.len, self.at) }
}

#[derive(Debug)]
struct Replace { from: String, to: String, positions: Vec<usize> }
impl Command for Replace {
    fn apply(&mut self, b: &mut Buffer) {
        self.positions = b.text.match_indices(&self.from).map(|(i, _)| i).collect();
        b.text = b.text.replace(&self.from, &self.to);
        b.cursor = b.cursor.min(b.text.len());
        while !b.text.is_char_boundary(b.cursor) { b.cursor -= 1; }
    }
    fn undo(&mut self, b: &mut Buffer) {
        let delta = self.to.len() as isize - self.from.len() as isize;
        for (k, &p) in self.positions.iter().enumerate() {
            let at = (p as isize + delta * k as isize) as usize;
            b.text.replace_range(at..at + self.to.len(), &self.from);
        }
    }
    fn name(&self) -> String { format!("replace {:?}->{:?} ({}x)", self.from, self.to, self.positions.len()) }
}

#[derive(Debug)]
struct Macro { name: String, steps: Vec<Box<dyn Command>> }
impl Command for Macro {
    fn apply(&mut self, b: &mut Buffer) { for s in self.steps.iter_mut() { s.apply(b); } }
    fn undo(&mut self, b: &mut Buffer) { for s in self.steps.iter_mut().rev() { s.undo(b); } }
    fn name(&self) -> String { format!("macro {} [{}]", self.name, self.steps.iter().map(|s| s.name()).collect::<Vec<_>>().join("; ")) }
}

#[derive(Default)]
struct Editor { buf: Buffer, undo: Vec<Box<dyn Command>>, redo: Vec<Box<dyn Command>>, log: Vec<String> }

impl Editor {
    fn exec(&mut self, mut c: Box<dyn Command>) {
        c.apply(&mut self.buf);
        self.redo.clear();
        if let Some(last) = self.undo.last_mut() {
            if last.merge(c.as_ref()) { self.log.push(format!("merged into {}", last.name())); return; }
        }
        self.log.push(c.name());
        self.undo.push(c);
    }
    fn type_str(&mut self, s: &str) { for ch in s.chars() { let at = self.buf.cursor; self.exec(Box::new(Insert { at, s: ch.to_string() })); } }
    fn backspace(&mut self) {
        let c = self.buf.cursor;
        if c == 0 { return; }
        let p = self.buf.prev_boundary(c);
        self.exec(Box::new(Delete { at: p, len: c - p, removed: None }));
    }
    fn undo(&mut self) -> bool {
        match self.undo.pop() { Some(mut c) => { c.undo(&mut self.buf); self.log.push(format!("undo {}", c.name())); self.redo.push(c); true } None => false }
    }
    fn redo(&mut self) -> bool {
        match self.redo.pop() { Some(mut c) => { c.apply(&mut self.buf); self.log.push(format!("redo {}", c.name())); self.undo.push(c); true } None => false }
    }
    fn left(&mut self, n: usize) { for _ in 0..n { self.buf.cursor = self.buf.prev_boundary(self.buf.cursor); } }
    fn right(&mut self, n: usize) { for _ in 0..n { self.buf.cursor = self.buf.next_boundary(self.buf.cursor); } }
    fn show(&self, tag: &str) {
        let (a, b) = self.buf.text.split_at(self.buf.cursor);
        println!("{:<14} {:?} | undo={} redo={}", tag, format!("{}|{}", a, b), self.undo.len(), self.redo.len());
    }
}

fn main() {
    let mut ed = Editor::default();
    ed.type_str("hello wörld");
    ed.show("typed:");
    ed.backspace();
    ed.backspace();
    ed.show("backspaced:");
    ed.type_str("ld!");
    ed.left(6);
    ed.show("moved left:");
    ed.type_str("big ");
    ed.show("inserted:");
    ed.exec(Box::new(Replace { from: "l".into(), to: "LL".into(), positions: vec![] }));
    ed.show("replaced:");
    ed.undo();
    ed.show("undo replace:");
    ed.undo();
    ed.undo();
    ed.show("undo x2:");
    ed.redo();
    ed.show("redo:");
    ed.exec(Box::new(Macro { name: "wrap".into(), steps: vec![Box::new(Insert { at: 0, s: "<".into() }), Box::new(Insert { at: 1, s: "<".into() }), Box::new(Delete { at: 2, len: 1, removed: None })] }));
    ed.show("macro:");
    println!("redo after new cmd: {}", ed.redo());
    while ed.undo() {}
    ed.show("undo all:");
    let mut n = 0;
    while ed.redo() { n += 1; }
    ed.show("redo all:");
    println!("redid {} commands", n);
    for (i, l) in ed.log.iter().enumerate() { println!("{:>2}. {}", i + 1, l); }
}
