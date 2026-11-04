// A regex engine: parser -> AST -> Thompson NFA, simulated with state sets.
// Also a backtracking matcher on the same AST for cross-checking and captures.
use std::collections::BTreeSet;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Class {
    Any,
    Char(char),
    Set { negated: bool, ranges: Vec<(char, char)> },
}

impl Class {
    fn matches(&self, c: char) -> bool {
        match self {
            Class::Any => c != '\n',
            Class::Char(x) => *x == c,
            Class::Set { negated, ranges } => ranges.iter().any(|&(lo, hi)| lo <= c && c <= hi) != *negated,
        }
    }
}

#[derive(Debug, Clone, PartialEq)]
enum Node {
    Empty,
    Atom(Class),
    Concat(Vec<Node>),
    Alt(Vec<Node>),
    Repeat { node: Box<Node>, min: u32, max: Option<u32>, greedy: bool },
    Group(Box<Node>, usize),
    Start,
    End,
}

impl fmt::Display for Node {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Node::Empty => write!(f, "ε"),
            Node::Atom(Class::Any) => write!(f, "."),
            Node::Atom(Class::Char(c)) => write!(f, "{}", c),
            Node::Atom(Class::Set { negated, ranges }) => {
                write!(f, "[{}", if *negated { "^" } else { "" })?;
                for (a, b) in ranges {
                    if a == b { write!(f, "{}", a)? } else { write!(f, "{}-{}", a, b)? }
                }
                write!(f, "]")
            }
            Node::Concat(v) => {
                for n in v { write!(f, "{}", n)?; }
                Ok(())
            }
            Node::Alt(v) => {
                write!(f, "(?:")?;
                for (i, n) in v.iter().enumerate() {
                    if i > 0 { write!(f, "|")?; }
                    write!(f, "{}", n)?;
                }
                write!(f, ")")
            }
            Node::Repeat { node, min, max, greedy } => {
                write!(f, "{}{{{},{}}}{}", node, min, max.map(|m| m.to_string()).unwrap_or_default(), if *greedy { "" } else { "?" })
            }
            Node::Group(n, i) => write!(f, "(#{} {})", i, n),
            Node::Start => write!(f, "^"),
            Node::End => write!(f, "$"),
        }
    }
}

struct ReParser<'a> {
    s: &'a [char],
    i: usize,
    groups: usize,
}

impl<'a> ReParser<'a> {
    fn parse(pat: &str) -> Result<(Node, usize), String> {
        let chars: Vec<char> = pat.chars().collect();
        let mut p = ReParser { s: &chars, i: 0, groups: 0 };
        let n = p.alt()?;
        if p.i < p.s.len() {
            return Err(format!("unexpected '{}' at {}", p.s[p.i], p.i));
        }
        Ok((n, p.groups))
    }
    fn peek(&self) -> Option<char> { self.s.get(self.i).copied() }
    fn alt(&mut self) -> Result<Node, String> {
        let mut alts = vec![self.concat()?];
        while self.peek() == Some('|') {
            self.i += 1;
            alts.push(self.concat()?);
        }
        Ok(if alts.len() == 1 { alts.pop().unwrap() } else { Node::Alt(alts) })
    }
    fn concat(&mut self) -> Result<Node, String> {
        let mut items = Vec::new();
        while let Some(c) = self.peek() {
            if c == '|' || c == ')' { break; }
            items.push(self.repeat()?);
        }
        Ok(match items.len() { 0 => Node::Empty, 1 => items.pop().unwrap(), _ => Node::Concat(items) })
    }
    fn number(&mut self) -> Option<u32> {
        let st = self.i;
        while self.peek().map_or(false, |c| c.is_ascii_digit()) { self.i += 1; }
        if st == self.i { None } else { self.s[st..self.i].iter().collect::<String>().parse().ok() }
    }
    fn repeat(&mut self) -> Result<Node, String> {
        let mut atom = self.atom()?;
        loop {
            let (min, max) = match self.peek() {
                Some('*') => { self.i += 1; (0, None) }
                Some('+') => { self.i += 1; (1, None) }
                Some('?') => { self.i += 1; (0, Some(1)) }
                Some('{') => {
                    let save = self.i;
                    self.i += 1;
                    let lo = match self.number() { Some(n) => n, None => { self.i = save; return Err(format!("bad repetition at {}", save)); } };
                    let hi = if self.peek() == Some(',') { self.i += 1; self.number() } else { Some(lo) };
                    if self.peek() != Some('}') { return Err("unclosed {".into()); }
                    self.i += 1;
                    if let Some(h) = hi { if h < lo { return Err(format!("bad range {{{},{}}}", lo, h)); } }
                    (lo, hi)
                }
                _ => return Ok(atom),
            };
            if matches!(atom, Node::Start | Node::End) { return Err("cannot repeat anchor".into()); }
            let greedy = if self.peek() == Some('?') { self.i += 1; false } else { true };
            atom = Node::Repeat { node: Box::new(atom), min, max, greedy };
        }
    }
    fn escape_class(c: char) -> Class {
        let digit = vec![('0', '9')];
        let word = vec![('0', '9'), ('A', 'Z'), ('_', '_'), ('a', 'z')];
        let space = vec![('\t', '\r'), (' ', ' ')];
        match c {
            'd' => Class::Set { negated: false, ranges: digit },
            'D' => Class::Set { negated: true, ranges: digit },
            'w' => Class::Set { negated: false, ranges: word },
            'W' => Class::Set { negated: true, ranges: word },
            's' => Class::Set { negated: false, ranges: space },
            'S' => Class::Set { negated: true, ranges: space },
            'n' => Class::Char('\n'),
            't' => Class::Char('\t'),
            other => Class::Char(other),
        }
    }
    fn atom(&mut self) -> Result<Node, String> {
        let c = self.peek().ok_or("unexpected end")?;
        self.i += 1;
        Ok(match c {
            '.' => Node::Atom(Class::Any),
            '^' => Node::Start,
            '$' => Node::End,
            '(' => {
                let capture = if self.s[self.i..].starts_with(&['?', ':']) { self.i += 2; false } else { true };
                let idx = if capture { self.groups += 1; self.groups } else { 0 };
                let inner = self.alt()?;
                if self.peek() != Some(')') { return Err("missing )".into()); }
                self.i += 1;
                if capture { Node::Group(Box::new(inner), idx) } else { inner }
            }
            '[' => {
                let negated = if self.peek() == Some('^') { self.i += 1; true } else { false };
                let mut ranges = Vec::new();
                let mut first = true;
                loop {
                    let c = self.peek().ok_or("unclosed [")?;
                    self.i += 1;
                    if c == ']' && !first { break; }
                    first = false;
                    let lo = if c == '\\' {
                        let e = self.peek().ok_or("bad escape")?;
                        self.i += 1;
                        match ReParser::escape_class(e) {
                            Class::Char(x) => x,
                            Class::Set { ranges: r, negated: false } => { ranges.extend(r); continue; }
                            _ => return Err("negated class in set".into()),
                        }
                    } else { c };
                    if self.peek() == Some('-') && self.s.get(self.i + 1).map_or(false, |&x| x != ']') {
                        let hi = self.s[self.i + 1];
                        self.i += 2;
                        if hi < lo { return Err(format!("bad class range {}-{}", lo, hi)); }
                        ranges.push((lo, hi));
                    } else {
                        ranges.push((lo, lo));
                    }
                }
                ranges.sort();
                Node::Atom(Class::Set { negated, ranges })
            }
            '\\' => {
                let e = self.peek().ok_or("trailing backslash")?;
                self.i += 1;
                Node::Atom(ReParser::escape_class(e))
            }
            '*' | '+' | '?' | '{' => return Err(format!("nothing to repeat at {}", self.i - 1)),
            ')' => return Err("unbalanced )".into()),
            c => Node::Atom(Class::Char(c)),
        })
    }
}

// ---------- Thompson NFA ----------
#[derive(Debug, Clone)]
enum Inst {
    Match,
    Char(Class),
    Split(usize, usize),
    Jmp(usize),
    Start,
    End,
}

struct Compiler { prog: Vec<Inst> }

impl Compiler {
    fn emit(&mut self, i: Inst) -> usize { self.prog.push(i); self.prog.len() - 1 }
    fn compile(&mut self, n: &Node) {
        match n {
            Node::Empty => {}
            Node::Atom(c) => { self.emit(Inst::Char(c.clone())); }
            Node::Start => { self.emit(Inst::Start); }
            Node::End => { self.emit(Inst::End); }
            Node::Group(inner, _) => self.compile(inner),
            Node::Concat(v) => v.iter().for_each(|x| self.compile(x)),
            Node::Alt(v) => {
                let mut jumps = Vec::new();
                for (i, alt) in v.iter().enumerate() {
                    if i + 1 < v.len() {
                        let split = self.emit(Inst::Split(0, 0));
                        self.compile(alt);
                        jumps.push(self.emit(Inst::Jmp(0)));
                        let next = self.prog.len();
                        self.prog[split] = Inst::Split(split + 1, next);
                    } else {
                        self.compile(alt);
                    }
                }
                let end = self.prog.len();
                for j in jumps { self.prog[j] = Inst::Jmp(end); }
            }
            Node::Repeat { node, min, max, greedy } => {
                for _ in 0..*min { self.compile(node); }
                match max {
                    None => {
                        let split = self.emit(Inst::Split(0, 0));
                        self.compile(node);
                        self.emit(Inst::Jmp(split));
                        let end = self.prog.len();
                        self.prog[split] = if *greedy { Inst::Split(split + 1, end) } else { Inst::Split(end, split + 1) };
                    }
                    Some(mx) => {
                        let mut splits = Vec::new();
                        for _ in *min..*mx {
                            splits.push(self.emit(Inst::Split(0, 0)));
                            self.compile(node);
                        }
                        let end = self.prog.len();
                        for s in splits {
                            self.prog[s] = if *greedy { Inst::Split(s + 1, end) } else { Inst::Split(end, s + 1) };
                        }
                    }
                }
            }
        }
    }
}

fn add_state(prog: &[Inst], set: &mut BTreeSet<usize>, pc: usize, pos: usize, len: usize) {
    if !set.insert(pc) { return; }
    match &prog[pc] {
        Inst::Split(a, b) => { add_state(prog, set, *a, pos, len); add_state(prog, set, *b, pos, len); }
        Inst::Jmp(a) => add_state(prog, set, *a, pos, len),
        Inst::Start if pos == 0 => add_state(prog, set, pc + 1, pos, len),
        Inst::End if pos == len => add_state(prog, set, pc + 1, pos, len),
        _ => {}
    }
}

fn nfa_full_match(prog: &[Inst], text: &[char]) -> bool {
    let mut cur = BTreeSet::new();
    add_state(prog, &mut cur, 0, 0, text.len());
    for (i, &c) in text.iter().enumerate() {
        let mut next = BTreeSet::new();
        for &pc in &cur {
            if let Inst::Char(cl) = &prog[pc] {
                if cl.matches(c) { add_state(prog, &mut next, pc + 1, i + 1, text.len()); }
            }
        }
        cur = next;
        if cur.is_empty() { return false; }
    }
    cur.iter().any(|&pc| matches!(prog[pc], Inst::Match))
}

// ---------- backtracking with captures ----------
type Caps = Vec<Option<(usize, usize)>>;

fn bt<'a>(n: &Node, t: &[char], pos: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
    match n {
        Node::Empty => k(pos, caps),
        Node::Atom(c) => pos < t.len() && c.matches(t[pos]) && k(pos + 1, caps),
        Node::Start => pos == 0 && k(pos, caps),
        Node::End => pos == t.len() && k(pos, caps),
        Node::Group(inner, idx) => {
            let idx = *idx;
            let saved = caps[idx];
            let ok = bt(inner, t, pos, caps, &mut |e, caps: &mut Caps| {
                let prev = caps[idx];
                caps[idx] = Some((pos, e));
                if k(e, caps) { true } else { caps[idx] = prev; false }
            });
            if !ok { caps[idx] = saved; }
            ok
        }
        Node::Concat(v) => concat_bt(v, t, pos, caps, k),
        Node::Alt(v) => v.iter().any(|a| bt(a, t, pos, caps, k)),
        Node::Repeat { node, min, max, greedy } => rep_bt(node, *min, *max, *greedy, t, pos, caps, k, 0),
    }
}

fn concat_bt(v: &[Node], t: &[char], pos: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool) -> bool {
    match v.split_first() {
        None => k(pos, caps),
        Some((first, rest)) => bt(first, t, pos, caps, &mut |p, caps: &mut Caps| concat_bt(rest, t, p, caps, k)),
    }
}

#[allow(clippy::too_many_arguments)]
fn rep_bt(n: &Node, min: u32, max: Option<u32>, greedy: bool, t: &[char], pos: usize, caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool, count: u32) -> bool {
    let can_more = max.map_or(true, |m| count < m);
    let mut more = |caps: &mut Caps, k: &mut dyn FnMut(usize, &mut Caps) -> bool| {
        can_more && bt(n, t, pos, caps, &mut |p, caps: &mut Caps| p != pos && rep_bt(n, min, max, greedy, t, p, caps, k, count + 1))
    };
    if count < min {
        return bt(n, t, pos, caps, &mut |p, caps: &mut Caps| rep_bt(n, min, max, greedy, t, p, caps, k, count + 1));
    }
    if greedy { more(caps, k) || k(pos, caps) } else { k(pos, caps) || more(caps, k) }
}

struct Regex { ast: Node, groups: usize, prog: Vec<Inst> }

impl Regex {
    fn new(p: &str) -> Result<Regex, String> {
        let (ast, groups) = ReParser::parse(p)?;
        let mut c = Compiler { prog: vec![] };
        c.compile(&ast);
        c.emit(Inst::Match);
        Ok(Regex { ast, groups, prog: c.prog })
    }
    fn is_full_match(&self, s: &str) -> bool {
        let t: Vec<char> = s.chars().collect();
        nfa_full_match(&self.prog, &t)
    }
    fn find(&self, s: &str) -> Option<(usize, usize, Vec<Option<String>>)> {
        let t: Vec<char> = s.chars().collect();
        for start in 0..=t.len() {
            let mut caps: Caps = vec![None; self.groups + 1];
            let mut end = None;
            if bt(&self.ast, &t, start, &mut caps, &mut |e, _| { end = Some(e); true }) {
                let e = end.unwrap();
                let strs = caps.iter().skip(1).map(|c| c.map(|(a, b)| t[a..b].iter().collect())).collect();
                return Some((start, e, strs));
            }
        }
        None
    }
    fn find_all(&self, s: &str) -> Vec<String> {
        let t: Vec<char> = s.chars().collect();
        let mut out = Vec::new();
        let mut start = 0;
        while start <= t.len() {
            let mut caps: Caps = vec![None; self.groups + 1];
            let mut end = None;
            if bt(&self.ast, &t, start, &mut caps, &mut |e, _| { end = Some(e); true }) {
                let e = end.unwrap();
                out.push(t[start..e].iter().collect());
                start = if e == start { e + 1 } else { e };
            } else {
                start += 1;
            }
        }
        out
    }
    fn replace_all(&self, s: &str, f: impl Fn(&[Option<String>]) -> String) -> String {
        let t: Vec<char> = s.chars().collect();
        let mut out = String::new();
        let mut i = 0;
        while i <= t.len() {
            let mut caps: Caps = vec![None; self.groups + 1];
            let mut end = None;
            if bt(&self.ast, &t, i, &mut caps, &mut |e, _| { end = Some(e); true }) && end != Some(i) {
                let e = end.unwrap();
                let strs: Vec<Option<String>> = caps.iter().map(|c| c.map(|(a, b)| t[a..b].iter().collect())).collect();
                out.push_str(&f(&strs));
                i = e;
            } else {
                if i < t.len() { out.push(t[i]); }
                i += 1;
            }
        }
        out
    }
}

fn main() {
    let cases: &[(&str, &[&str])] = &[
        ("a*b", &["b", "aaab", "aaa", "ab", ""]),
        ("(a|b)*abb", &["abb", "aababb", "abab", "babb"]),
        ("colou?r", &["color", "colour", "colouur"]),
        ("[a-c]{2,3}x", &["abx", "abcx", "ax", "abcax"]),
        ("\\d{3}-\\d{4}", &["555-1234", "55-1234", "555-12345"]),
        ("[^aeiou]+", &["rhythm", "strength", ""]),
        ("(?:ab|cd)+e?", &["ab", "cdabe", "abc", "e"]),
        ("x{0}y", &["y", "xy"]),
        ("(a+)+b", &["aaaaaaaaaaaaaaaaaaaaab", "aaaaaaaaaaaaaaaaaaaa"]),
        ("é.ü", &["éxü", "é\nü", "eü"]),
        ("[\\w.]+@\\w+\\.(com|org)", &["a.b_c@site.org", "x@y.net"]),
    ];
    for (pat, inputs) in cases {
        let re = Regex::new(pat).unwrap();
        let res: Vec<String> = inputs.iter().map(|s| format!("{:?}={}", s, if re.is_full_match(s) { "Y" } else { "n" })).collect();
        println!("{:<22} prog={:<3} {}", pat, re.prog.len(), res.join(" "));
    }

    for bad in ["(ab", "a)", "*a", "a{3,1}", "[z-a]", "a{x}", "^*", "[abc", "a\\"] {
        match Regex::new(bad) {
            Ok(r) => println!("{:?} unexpectedly ok: {}", bad, r.ast),
            Err(e) => println!("{:<8} error: {}", format!("{:?}", bad), e),
        }
    }

    let date = Regex::new("(\\d{4})-(\\d\\d)-(\\d\\d)").unwrap();
    println!("ast: {}", date.ast);
    for s in ["on 2024-02-29 we", "none here", "1999-12-31/2000-01-01"] {
        println!("find {:?} -> {:?}", s, date.find(s));
    }
    println!("all dates: {:?}", date.find_all("2001-01-01, 2002-02-02 and 03-03-03, 2004-04-04"));
    println!("swap: {}", date.replace_all("from 2024-03-15 to 2024-12-01.", |c| format!(
        "{}/{}/{}", c[3].as_deref().unwrap(), c[2].as_deref().unwrap(), c[1].as_deref().unwrap())));

    let lazy = Regex::new("<(.+?)>").unwrap();
    let greedy = Regex::new("<(.+)>").unwrap();
    let html = "<b>bold</b> and <i>it</i>";
    println!("lazy:   {:?}", lazy.find_all(html));
    println!("greedy: {:?}", greedy.find_all(html));
    let words = Regex::new("\\b?[A-Za-z]+").unwrap();
    println!("words: {:?}", words.find_all("Hello, wörld! It's 9am b4 lunch."));
    let opt = Regex::new("(a)?(b)?c").unwrap();
    for s in ["c", "ac", "bc", "abc", "xxabcx"] {
        println!("{:?} -> {:?}", s, opt.find(s).map(|x| x.2));
    }
    let anch = Regex::new("^(\\w+)|(\\w+)$").unwrap();
    println!("anchored: {:?}", anch.find_all("first middle last"));
    let empty = Regex::new("x*").unwrap();
    println!("empty matches: {:?}", empty.find_all("axxbx"));
    println!("squash: {}", Regex::new("\\s+").unwrap().replace_all("  lots   of \t space\n here ", |_| " ".to_string()));
    let nested = Regex::new("((a)(b(c)?))+").unwrap();
    println!("nested: {:?}", nested.find("xxababcab"));
}
