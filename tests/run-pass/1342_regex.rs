// Regex engine: parser -> AST -> Thompson NFA (Pike VM with captures) + backtracking
// matcher, cross-checked against each other.
use std::collections::HashSet;
use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
enum ClassItem {
    Ch(char),
    Range(char, char),
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Empty,
    Lit(char),
    Any,
    Class { neg: bool, items: Vec<ClassItem> },
    Start,
    End,
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat { node: Box<Node>, min: u32, max: Option<u32>, greedy: bool },
    Group(Option<usize>, Box<Node>),
}

#[derive(Debug, PartialEq)]
enum ParseError {
    UnexpectedEof,
    Unbalanced(usize),
    BadRepeat(char),
    BadRange(char, char),
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ParseError::UnexpectedEof => write!(f, "unexpected end of pattern"),
            ParseError::Unbalanced(p) => write!(f, "unbalanced parenthesis at {}", p),
            ParseError::BadRepeat(c) => write!(f, "nothing to repeat before '{}'", c),
            ParseError::BadRange(a, b) => write!(f, "invalid range {}-{}", a, b),
        }
    }
}

struct ReParser<'a> {
    it: Peekable<Chars<'a>>,
    pos: usize,
    ngroups: usize,
    depth: usize,
}

impl<'a> ReParser<'a> {
    fn bump(&mut self) -> Option<char> {
        let c = self.it.next();
        if c.is_some() {
            self.pos += 1;
        }
        c
    }

    fn parse(pat: &'a str) -> Result<(Node, usize), ParseError> {
        let mut p = ReParser { it: pat.chars().peekable(), pos: 0, ngroups: 1, depth: 0 };
        let n = p.alt()?;
        if p.it.peek().is_some() {
            return Err(ParseError::Unbalanced(p.pos));
        }
        Ok((Node::Group(Some(0), Box::new(n)), p.ngroups))
    }

    fn alt(&mut self) -> Result<Node, ParseError> {
        let mut alts = vec![self.concat()?];
        while self.it.peek() == Some(&'|') {
            self.bump();
            alts.push(self.concat()?);
        }
        Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Node::Alt(alts) })
    }

    fn concat(&mut self) -> Result<Node, ParseError> {
        let mut items = Vec::new();
        loop {
            match self.it.peek() {
                None | Some('|') => break,
                Some(')') => {
                    if self.depth == 0 {
                        return Err(ParseError::Unbalanced(self.pos));
                    }
                    break;
                }
                _ => {}
            }
            let atom = self.atom()?;
            let atom = self.repeat(atom)?;
            items.push(atom);
        }
        Ok(match items.len() {
            0 => Node::Empty,
            1 => items.pop().unwrap(),
            _ => Node::Concat(items),
        })
    }

    fn number(&mut self) -> Option<u32> {
        let mut n: Option<u32> = None;
        while let Some(c) = self.it.peek().copied() {
            if let Some(d) = c.to_digit(10) {
                n = Some(n.unwrap_or(0) * 10 + d);
                self.bump();
            } else {
                break;
            }
        }
        n
    }

    fn repeat(&mut self, mut atom: Node) -> Result<Node, ParseError> {
        loop {
            let (min, max) = match self.it.peek() {
                Some('*') => (0, None),
                Some('+') => (1, None),
                Some('?') => (0, Some(1)),
                Some('{') => {
                    self.bump();
                    let lo = self.number().ok_or(ParseError::BadRepeat('{'))?;
                    let hi = if self.it.peek() == Some(&',') {
                        self.bump();
                        self.number()
                    } else {
                        Some(lo)
                    };
                    if self.bump() != Some('}') {
                        return Err(ParseError::BadRepeat('{'));
                    }
                    let greedy = !self.eat('?');
                    atom = Node::Repeat { node: Box::new(atom), min: lo, max: hi, greedy };
                    continue;
                }
                _ => return Ok(atom),
            };
            let c = self.bump().unwrap();
            if matches!(atom, Node::Empty | Node::Start | Node::End) {
                return Err(ParseError::BadRepeat(c));
            }
            let greedy = !self.eat('?');
            atom = Node::Repeat { node: Box::new(atom), min, max, greedy };
        }
    }

    fn eat(&mut self, c: char) -> bool {
        if self.it.peek() == Some(&c) {
            self.bump();
            true
        } else {
            false
        }
    }

    fn escape(&mut self) -> Result<Node, ParseError> {
        let c = self.bump().ok_or(ParseError::UnexpectedEof)?;
        Ok(match c {
            'd' => Node::Class { neg: false, items: vec![ClassItem::Range('0', '9')] },
            'D' => Node::Class { neg: true, items: vec![ClassItem::Range('0', '9')] },
            'w' => Node::Class { neg: false, items: word_items() },
            'W' => Node::Class { neg: true, items: word_items() },
            's' => Node::Class { neg: false, items: " \t\n\r".chars().map(ClassItem::Ch).collect() },
            'n' => Node::Lit('\n'),
            't' => Node::Lit('\t'),
            other => Node::Lit(other),
        })
    }

    fn atom(&mut self) -> Result<Node, ParseError> {
        let c = self.bump().ok_or(ParseError::UnexpectedEof)?;
        match c {
            '(' => {
                let idx = if self.eat('?') {
                    if !self.eat(':') {
                        return Err(ParseError::BadRepeat('?'));
                    }
                    None
                } else {
                    let i = self.ngroups;
                    self.ngroups += 1;
                    Some(i)
                };
                self.depth += 1;
                let inner = self.alt()?;
                self.depth -= 1;
                if self.bump() != Some(')') {
                    return Err(ParseError::Unbalanced(self.pos));
                }
                Ok(Node::Group(idx, Box::new(inner)))
            }
            '[' => {
                let neg = self.eat('^');
                let mut items = Vec::new();
                let mut first = true;
                loop {
                    let c = self.bump().ok_or(ParseError::UnexpectedEof)?;
                    if c == ']' && !first {
                        break;
                    }
                    first = false;
                    let c = if c == '\\' { self.bump().ok_or(ParseError::UnexpectedEof)? } else { c };
                    if self.it.peek() == Some(&'-') {
                        let mut look = self.it.clone();
                        look.next();
                        if look.peek().map_or(false, |&n| n != ']') {
                            self.bump();
                            let hi = self.bump().unwrap();
                            if hi < c {
                                return Err(ParseError::BadRange(c, hi));
                            }
                            items.push(ClassItem::Range(c, hi));
                            continue;
                        }
                    }
                    items.push(ClassItem::Ch(c));
                }
                Ok(Node::Class { neg, items })
            }
            '.' => Ok(Node::Any),
            '^' => Ok(Node::Start),
            '$' => Ok(Node::End),
            '\\' => self.escape(),
            '*' | '+' | '?' => Err(ParseError::BadRepeat(c)),
            c => Ok(Node::Lit(c)),
        }
    }
}

fn word_items() -> Vec<ClassItem> {
    vec![ClassItem::Range('a', 'z'), ClassItem::Range('A', 'Z'), ClassItem::Range('0', '9'), ClassItem::Ch('_')]
}

fn class_matches(neg: bool, items: &[ClassItem], c: char) -> bool {
    let hit = items.iter().any(|it| match *it {
        ClassItem::Ch(x) => x == c,
        ClassItem::Range(a, b) => (a..=b).contains(&c),
    });
    hit != neg
}

// ---------------- Pike VM ----------------
#[derive(Debug, Clone)]
enum Inst {
    Char(char),
    Any,
    Class(bool, Vec<ClassItem>),
    Split(usize, usize),
    Jmp(usize),
    Save(usize),
    AssertStart,
    AssertEnd,
    Match,
}

struct Program {
    insts: Vec<Inst>,
    nslots: usize,
}

struct Compiler {
    insts: Vec<Inst>,
}

impl Compiler {
    fn emit(&mut self, i: Inst) -> usize {
        self.insts.push(i);
        self.insts.len() - 1
    }
    fn patch_split(&mut self, at: usize, a: usize, b: usize) {
        self.insts[at] = Inst::Split(a, b);
    }
    fn c(&mut self, n: &Node) {
        match n {
            Node::Empty => {}
            Node::Lit(c) => {
                self.emit(Inst::Char(*c));
            }
            Node::Any => {
                self.emit(Inst::Any);
            }
            Node::Class { neg, items } => {
                self.emit(Inst::Class(*neg, items.clone()));
            }
            Node::Start => {
                self.emit(Inst::AssertStart);
            }
            Node::End => {
                self.emit(Inst::AssertEnd);
            }
            Node::Concat(v) => v.iter().for_each(|x| self.c(x)),
            Node::Alt(v) => {
                let mut jumps = vec![];
                for (i, alt) in v.iter().enumerate() {
                    if i + 1 < v.len() {
                        let sp = self.emit(Inst::Split(0, 0));
                        self.c(alt);
                        jumps.push(self.emit(Inst::Jmp(0)));
                        let next = self.insts.len();
                        self.patch_split(sp, sp + 1, next);
                    } else {
                        self.c(alt);
                    }
                }
                let end = self.insts.len();
                for j in jumps {
                    self.insts[j] = Inst::Jmp(end);
                }
            }
            Node::Group(idx, inner) => {
                if let Some(i) = idx {
                    self.emit(Inst::Save(2 * i));
                }
                self.c(inner);
                if let Some(i) = idx {
                    self.emit(Inst::Save(2 * i + 1));
                }
            }
            Node::Repeat { node, min, max, greedy } => {
                for _ in 0..*min {
                    self.c(node);
                }
                match max {
                    None => {
                        let sp = self.emit(Inst::Split(0, 0));
                        self.c(node);
                        self.emit(Inst::Jmp(sp));
                        let end = self.insts.len();
                        if *greedy {
                            self.patch_split(sp, sp + 1, end);
                        } else {
                            self.patch_split(sp, end, sp + 1);
                        }
                    }
                    Some(mx) => {
                        let mut splits = vec![];
                        for _ in *min..*mx {
                            splits.push(self.emit(Inst::Split(0, 0)));
                            self.c(node);
                        }
                        let end = self.insts.len();
                        for sp in splits {
                            if *greedy {
                                self.patch_split(sp, sp + 1, end);
                            } else {
                                self.patch_split(sp, end, sp + 1);
                            }
                        }
                    }
                }
            }
        }
    }
}

fn compile(n: &Node, ngroups: usize) -> Program {
    let mut c = Compiler { insts: vec![] };
    c.c(n);
    c.emit(Inst::Match);
    Program { insts: c.insts, nslots: ngroups * 2 }
}

type Caps = Vec<Option<usize>>;

fn add_thread(prog: &Program, list: &mut Vec<(usize, Caps)>, seen: &mut [bool], pc: usize, pos: usize, len: usize, caps: Caps) {
    if seen[pc] {
        return;
    }
    seen[pc] = true;
    match &prog.insts[pc] {
        Inst::Jmp(t) => add_thread(prog, list, seen, *t, pos, len, caps),
        Inst::Split(a, b) => {
            add_thread(prog, list, seen, *a, pos, len, caps.clone());
            add_thread(prog, list, seen, *b, pos, len, caps);
        }
        Inst::Save(s) => {
            let mut c = caps;
            c[*s] = Some(pos);
            add_thread(prog, list, seen, pc + 1, pos, len, c);
        }
        Inst::AssertStart => {
            if pos == 0 {
                add_thread(prog, list, seen, pc + 1, pos, len, caps);
            }
        }
        Inst::AssertEnd => {
            if pos == len {
                add_thread(prog, list, seen, pc + 1, pos, len, caps);
            }
        }
        _ => list.push((pc, caps)),
    }
}

fn pike_at(prog: &Program, text: &[char], start: usize) -> Option<Caps> {
    let mut clist: Vec<(usize, Caps)> = Vec::new();
    let mut seen = vec![false; prog.insts.len()];
    add_thread(prog, &mut clist, &mut seen, 0, start, text.len(), vec![None; prog.nslots]);
    let mut matched: Option<Caps> = None;
    let mut pos = start;
    loop {
        if clist.is_empty() {
            break;
        }
        let mut nlist = Vec::new();
        let mut seen = vec![false; prog.insts.len()];
        let ch = text.get(pos).copied();
        for (pc, caps) in clist.drain(..) {
            let ok = match (&prog.insts[pc], ch) {
                (Inst::Match, _) => {
                    matched = Some(caps);
                    break; // lower priority threads are cut off
                }
                (Inst::Char(c), Some(x)) => *c == x,
                (Inst::Any, Some(x)) => x != '\n',
                (Inst::Class(neg, items), Some(x)) => class_matches(*neg, items, x),
                _ => false,
            };
            if ok {
                add_thread(prog, &mut nlist, &mut seen, pc + 1, pos + 1, text.len(), caps);
            }
        }
        if ch.is_none() {
            break;
        }
        clist = nlist;
        pos += 1;
    }
    matched
}

// --------------- Backtracker over AST ---------------
struct Backtracker<'t> {
    text: &'t [char],
    caps: Caps,
    steps: u64,
}

impl<'t> Backtracker<'t> {
    // continuation-passing: match node `n` at `pos`, then call k
    fn m(&mut self, n: &Node, pos: usize, k: &mut dyn FnMut(&mut Self, usize) -> bool) -> bool {
        self.steps += 1;
        match n {
            Node::Empty => k(self, pos),
            Node::Lit(c) => self.text.get(pos) == Some(c) && k(self, pos + 1),
            Node::Any => matches!(self.text.get(pos), Some(&c) if c != '\n') && k(self, pos + 1),
            Node::Class { neg, items } => match self.text.get(pos) {
                Some(&c) if class_matches(*neg, items, c) => k(self, pos + 1),
                _ => false,
            },
            Node::Start => pos == 0 && k(self, pos),
            Node::End => pos == self.text.len() && k(self, pos),
            Node::Concat(v) => self.seq(v, pos, k),
            Node::Alt(v) => {
                for alt in v {
                    if self.m(alt, pos, k) {
                        return true;
                    }
                }
                false
            }
            Node::Group(idx, inner) => match idx {
                None => self.m(inner, pos, k),
                Some(i) => {
                    let i = *i;
                    let old = (self.caps[2 * i], self.caps[2 * i + 1]);
                    let ok = self.m(inner, pos, &mut |s: &mut Self, p| {
                        let save = (s.caps[2 * i], s.caps[2 * i + 1]);
                        s.caps[2 * i] = Some(pos);
                        s.caps[2 * i + 1] = Some(p);
                        if k(s, p) {
                            true
                        } else {
                            s.caps[2 * i] = save.0;
                            s.caps[2 * i + 1] = save.1;
                            false
                        }
                    });
                    if !ok {
                        self.caps[2 * i] = old.0;
                        self.caps[2 * i + 1] = old.1;
                    }
                    ok
                }
            },
            Node::Repeat { node, min, max, greedy } => self.rep(node, *min, *max, *greedy, 0, pos, usize::MAX, k),
        }
    }

    fn seq(&mut self, v: &[Node], pos: usize, k: &mut dyn FnMut(&mut Self, usize) -> bool) -> bool {
        match v.split_first() {
            None => k(self, pos),
            Some((first, rest)) => self.m(first, pos, &mut |s: &mut Self, p| s.seq(rest, p, k)),
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn rep(&mut self, n: &Node, min: u32, max: Option<u32>, greedy: bool, count: u32, pos: usize, last: usize, k: &mut dyn FnMut(&mut Self, usize) -> bool) -> bool {
        let can_more = max.map_or(true, |m| count < m) && last != pos;
        let mut more = |s: &mut Self, k: &mut dyn FnMut(&mut Self, usize) -> bool| -> bool {
            can_more && s.m(n, pos, &mut |s2: &mut Self, p| s2.rep(n, min, max, greedy, count + 1, p, pos, k))
        };
        if count < min {
            return more(self, k);
        }
        if greedy {
            more(self, k) || k(self, pos)
        } else {
            k(self, pos) || more(self, k)
        }
    }
}

struct Regex {
    src: String,
    ast: Node,
    prog: Program,
    ngroups: usize,
}

#[derive(Debug, PartialEq, Clone)]
struct Match {
    start: usize,
    end: usize,
    groups: Vec<Option<(usize, usize)>>,
}

impl Regex {
    fn new(p: &str) -> Result<Regex, ParseError> {
        let (ast, ngroups) = ReParser::parse(p)?;
        let prog = compile(&ast, ngroups);
        Ok(Regex { src: p.to_string(), ast, prog, ngroups })
    }

    fn caps_to_match(&self, caps: &Caps) -> Match {
        let groups = (0..self.ngroups)
            .map(|g| match (caps[2 * g], caps[2 * g + 1]) {
                (Some(a), Some(b)) => Some((a, b)),
                _ => None,
            })
            .collect::<Vec<_>>();
        let (start, end) = groups[0].unwrap();
        Match { start, end, groups }
    }

    fn find_pike(&self, text: &[char], from: usize) -> Option<Match> {
        (from..=text.len()).find_map(|s| pike_at(&self.prog, text, s)).map(|c| self.caps_to_match(&c))
    }

    fn find_bt(&self, text: &[char], from: usize) -> (Option<Match>, u64) {
        let mut steps = 0;
        for s in from..=text.len() {
            let mut bt = Backtracker { text, caps: vec![None; self.ngroups * 2], steps: 0 };
            let ok = bt.m(&self.ast, s, &mut |_, _| true);
            steps += bt.steps;
            if ok {
                return (Some(self.caps_to_match(&bt.caps)), steps);
            }
        }
        (None, steps)
    }

    fn find_all(&self, s: &str) -> Vec<String> {
        let text: Vec<char> = s.chars().collect();
        let mut out = vec![];
        let mut at = 0;
        while at <= text.len() {
            match self.find_pike(&text, at) {
                Some(m) => {
                    out.push(text[m.start..m.end].iter().collect());
                    at = if m.end == m.start { m.end + 1 } else { m.end };
                }
                None => break,
            }
        }
        out
    }

    fn replace_all(&self, s: &str, rep: impl Fn(&[Option<String>]) -> String) -> String {
        let text: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut at = 0;
        let mut last = 0;
        while at <= text.len() {
            let Some(m) = self.find_pike(&text, at) else { break };
            out.extend(&text[last..m.start]);
            let gs: Vec<Option<String>> = m.groups.iter().map(|g| g.map(|(a, b)| text[a..b].iter().collect())).collect();
            out.push_str(&rep(&gs));
            last = m.end;
            at = if m.end == m.start {
                if m.end < text.len() {
                    out.push(text[m.end]);
                }
                last = m.end + 1;
                m.end + 1
            } else {
                m.end
            };
        }
        if last < text.len() {
            out.extend(&text[last..]);
        }
        out
    }
}

fn show_match(text: &[char], m: &Option<Match>) -> String {
    match m {
        None => "no match".to_string(),
        Some(m) => {
            let gs: Vec<String> = m
                .groups
                .iter()
                .map(|g| match g {
                    Some((a, b)) => format!("{:?}", text[*a..*b].iter().collect::<String>()),
                    None => "-".to_string(),
                })
                .collect();
            format!("[{}..{}] {}", m.start, m.end, gs.join(" "))
        }
    }
}

fn main() {
    let cases: &[(&str, &[&str])] = &[
        ("a(b|c)*d", &["abcbd", "ad", "xxabd", "abce"]),
        ("^(\\d+)-(\\d+)$", &["12-345", "12-", "a12-3"]),
        ("(a+)(a+)", &["aaaa", "a"]),
        ("(a+?)(a+)", &["aaaa"]),
        ("colou?r", &["color", "colour", "colouur"]),
        ("[a-z]+@[a-z]+\\.(com|org)", &["mail bob@example.com now", "x@y.net", "me@site.org"]),
        ("(\\w+)\\s(\\w+)", &["hello world", "  foo   bar"]),
        ("x{2,3}", &["x", "xx", "xxxxx"]),
        ("(ab){2}", &["abab", "ab", "cababab"]),
        ("[^aeiou ]+", &["the quick brown"]),
        ("(a|ab)(c|bcd)(d*)", &["abcd"]),
        ("(a*)*b", &["aaab", "aaaa"]),
        ("", &["", "abc"]),
        ("a.c", &["abc", "a\nc", "ac"]),
        ("[-a]+", &["--a-b"]),
        ("(x)?(y)", &["y", "xy"]),
    ];
    let mut agree = 0;
    let mut total = 0;
    for (pat, inputs) in cases {
        let re = match Regex::new(pat) {
            Ok(r) => r,
            Err(e) => {
                println!("{:?}: error {}", pat, e);
                continue;
            }
        };
        println!("/{}/ insts={} groups={}", re.src, re.prog.insts.len(), re.ngroups);
        for inp in inputs.iter() {
            let text: Vec<char> = inp.chars().collect();
            let a = re.find_pike(&text, 0);
            let (b, steps) = re.find_bt(&text, 0);
            total += 1;
            let same = a == b;
            if same {
                agree += 1;
            }
            println!("  {:<22} pike: {:<40} bt-steps={} {}", format!("{:?}", inp), show_match(&text, &a), steps, if same { "" } else { "MISMATCH" });
            if !same {
                println!("    backtracker: {}", show_match(&text, &b));
            }
        }
    }
    println!("agree {}/{}", agree, total);

    for bad in ["(ab", "ab)", "*a", "a{x}", "[z-a]", "[abc", "a\\"] {
        match Regex::new(bad) {
            Ok(_) => println!("{:?} parsed?!", bad),
            Err(e) => println!("{:?} -> {} / {:?}", bad, e, e),
        }
    }

    let re = Regex::new("\\d+").unwrap();
    println!("{:?}", re.find_all("a1 b22 c333 4444d"));
    let re = Regex::new("(\\w+)=(\\w+)").unwrap();
    let swapped = re.replace_all("a=1, bb=22, ccc=333", |g| format!("{}={}", g[2].as_deref().unwrap_or(""), g[1].as_deref().unwrap_or("")));
    println!("{}", swapped);
    let re = Regex::new("x*").unwrap();
    println!("{:?}", re.replace_all("abc", |_| "-".to_string()));
    println!("{:?}", re.find_all("axxbx"));

    // pathological for backtracking, linear for pike
    let n = 16;
    let pat = format!("{}{}", "a?".repeat(n), "a".repeat(n));
    let re = Regex::new(&pat).unwrap();
    let text: Vec<char> = "a".repeat(n).chars().collect();
    let p = re.find_pike(&text, 0);
    let (b, steps) = re.find_bt(&text, 0);
    println!("pathological n={} pike={} bt={} steps={}", n, show_match(&text, &p).len(), b.is_some(), steps);

    // unicode text
    let re = Regex::new("é+(.)").unwrap();
    let t: Vec<char> = "caféééx!".chars().collect();
    println!("{}", show_match(&t, &re.find_pike(&t, 0)));

    let mut uniq: HashSet<String> = HashSet::new();
    for w in Regex::new("[a-z]+").unwrap().find_all("the cat and the hat and the bat") {
        uniq.insert(w);
    }
    let mut u: Vec<_> = uniq.into_iter().collect();
    u.sort_by(|a, b| b.len().cmp(&a.len()).then(a.cmp(b)));
    println!("{:?}", u);
}
