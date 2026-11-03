// Thompson NFA regex: parse to AST, compile to NFA states, simulate with
// state sets. Supports literals, '.', '*', '+', '?', '|', groups, classes.
use std::collections::BTreeSet;

#[derive(Debug, Clone)]
enum Ast {
    Char(char),
    Any,
    Class(Vec<(char, char)>, bool),
    Concat(Vec<Ast>),
    Alt(Box<Ast>, Box<Ast>),
    Star(Box<Ast>),
    Plus(Box<Ast>),
    Quest(Box<Ast>),
}

struct Parser<'a> {
    chars: std::iter::Peekable<std::str::Chars<'a>>,
}

impl<'a> Parser<'a> {
    fn parse_alt(&mut self) -> Result<Ast, String> {
        let left = self.parse_concat()?;
        if self.chars.peek() == Some(&'|') {
            self.chars.next();
            let right = self.parse_alt()?;
            return Ok(Ast::Alt(Box::new(left), Box::new(right)));
        }
        Ok(left)
    }
    fn parse_concat(&mut self) -> Result<Ast, String> {
        let mut items = Vec::new();
        while let Some(&c) = self.chars.peek() {
            if c == '|' || c == ')' {
                break;
            }
            let atom = self.parse_atom()?;
            let atom = match self.chars.peek() {
                Some('*') => { self.chars.next(); Ast::Star(Box::new(atom)) }
                Some('+') => { self.chars.next(); Ast::Plus(Box::new(atom)) }
                Some('?') => { self.chars.next(); Ast::Quest(Box::new(atom)) }
                _ => atom,
            };
            items.push(atom);
        }
        Ok(if items.len() == 1 { items.pop().unwrap() } else { Ast::Concat(items) })
    }
    fn parse_atom(&mut self) -> Result<Ast, String> {
        match self.chars.next() {
            Some('(') => {
                let inner = self.parse_alt()?;
                match self.chars.next() {
                    Some(')') => Ok(inner),
                    other => Err(format!("expected ')', got {:?}", other)),
                }
            }
            Some('.') => Ok(Ast::Any),
            Some('[') => {
                let mut neg = false;
                if self.chars.peek() == Some(&'^') {
                    neg = true;
                    self.chars.next();
                }
                let mut ranges = Vec::new();
                loop {
                    let c = self.chars.next().ok_or("unterminated class")?;
                    if c == ']' {
                        break;
                    }
                    if self.chars.peek() == Some(&'-') {
                        self.chars.next();
                        let hi = self.chars.next().ok_or("bad range")?;
                        ranges.push((c, hi));
                    } else {
                        ranges.push((c, c));
                    }
                }
                Ok(Ast::Class(ranges, neg))
            }
            Some('\\') => self.chars.next().map(Ast::Char).ok_or_else(|| "trailing backslash".to_string()),
            Some(c) if "*+?".contains(c) => Err(format!("nothing to repeat before {}", c)),
            Some(c) => Ok(Ast::Char(c)),
            None => Err("unexpected end".into()),
        }
    }
}

struct Pred(Box<dyn Fn(char) -> bool>);

#[derive(Debug)]
enum State {
    Match(Pred),
    Split(usize, usize),
    Jump(usize),
    Accept,
}

impl std::fmt::Debug for Pred {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "<pred>")
    }
}

struct Nfa {
    states: Vec<Option<State>>,
}

impl Nfa {
    fn push(&mut self, s: State) -> usize {
        self.states.push(Some(s));
        self.states.len() - 1
    }
    fn placeholder(&mut self) -> usize {
        self.states.push(None);
        self.states.len() - 1
    }
    // compile `ast` so that it starts at returned index and continues to `next`
    fn compile(&mut self, ast: &Ast, next: usize) -> usize {
        match ast {
            Ast::Char(c) => {
                let c = *c;
                let p = self.push(State::Match(Pred(Box::new(move |x| x == c))));
                self.link(p, next);
                p
            }
            Ast::Any => {
                let p = self.push(State::Match(Pred(Box::new(|_| true))));
                self.link(p, next);
                p
            }
            Ast::Class(r, neg) => {
                let (r, neg) = (r.clone(), *neg);
                let p = self.push(State::Match(Pred(Box::new(move |x| r.iter().any(|&(a, b)| a <= x && x <= b) != neg))));
                self.link(p, next);
                p
            }
            Ast::Concat(items) => items.iter().rev().fold(next, |n, a| self.compile(a, n)),
            Ast::Alt(a, b) => {
                let x = self.compile(a, next);
                let y = self.compile(b, next);
                self.push(State::Split(x, y))
            }
            Ast::Quest(a) => {
                let x = self.compile(a, next);
                self.push(State::Split(x, next))
            }
            Ast::Star(a) => {
                let split = self.placeholder();
                let body = self.compile(a, split);
                self.states[split] = Some(State::Split(body, next));
                split
            }
            Ast::Plus(a) => {
                let split = self.placeholder();
                let body = self.compile(a, split);
                self.states[split] = Some(State::Split(body, next));
                body
            }
        }
    }
    // Match states store their successor in a parallel jump state
    fn link(&mut self, p: usize, next: usize) {
        let j = self.push(State::Jump(next));
        assert_eq!(j, p + 1);
    }
    fn closure(&self, s: usize, set: &mut BTreeSet<usize>) {
        if !set.insert(s) {
            return;
        }
        match self.states[s].as_ref().unwrap() {
            State::Split(a, b) => {
                self.closure(*a, set);
                self.closure(*b, set);
            }
            State::Jump(n) => self.closure(*n, set),
            _ => {}
        }
    }
    fn is_match(&self, start: usize, text: &str) -> bool {
        let mut cur = BTreeSet::new();
        self.closure(start, &mut cur);
        for c in text.chars() {
            let mut nxt = BTreeSet::new();
            for &s in &cur {
                if let Some(State::Match(pred)) = &self.states[s] {
                    if (pred.0)(c) {
                        self.closure(s + 1, &mut nxt);
                    }
                }
            }
            cur = nxt;
            if cur.is_empty() {
                return false;
            }
        }
        cur.iter().any(|&s| matches!(self.states[s], Some(State::Accept)))
    }
}

fn compile(pat: &str) -> Result<(Nfa, usize), String> {
    let mut p = Parser { chars: pat.chars().peekable() };
    let ast = p.parse_alt()?;
    if let Some(c) = p.chars.next() {
        return Err(format!("unexpected {:?}", c));
    }
    let mut nfa = Nfa { states: Vec::new() };
    let acc = nfa.push(State::Accept);
    let start = nfa.compile(&ast, acc);
    Ok((nfa, start))
}

fn main() {
    let cases: &[(&str, &[&str])] = &[
        ("abc", &["abc", "ab", "abcd", ""]),
        ("a*b", &["b", "ab", "aaab", "aaa", "ba"]),
        ("(ab|cd)+e?", &["ab", "cdab", "abe", "e", "abcde", "abcdee"]),
        ("[a-c]x[^0-9]", &["axz", "cx_", "dxz", "bx5", "ax"]),
        ("h.llo|wor(l|ld)", &["hello", "hallo", "world", "worl", "worldd"]),
        ("(a|b)*abb", &["abb", "aabb", "babb", "ababab", "bbbbabb"]),
        ("\\*+\\.", &["*.", "***.", ".", "*x"]),
        ("(x?)*y", &["y", "xy", "xxxy", "xx"]),
    ];
    for (pat, inputs) in cases {
        let (nfa, start) = compile(pat).unwrap();
        let res: Vec<String> = inputs
            .iter()
            .map(|s| format!("{}:{}", s, if nfa.is_match(start, s) { 'Y' } else { 'n' }))
            .collect();
        println!("{:<16} states={:<3} {}", pat, nfa.states.len(), res.join(" "));
    }
    for bad in ["(ab", "*a", "a)", "[ab", "a\\"] {
        match compile(bad) {
            Ok(_) => println!("{:?}: ok?", bad),
            Err(e) => println!("{:?}: error: {}", bad, e),
        }
    }
    let (nfa, start) = compile("[a-z]+@[a-z]+\\.(com|org)").unwrap();
    let emails = ["bob@example.com", "x@y.org", "bad@site.net", "@a.com", "q@w.comm"];
    let ok: Vec<_> = emails.iter().filter(|e| nfa.is_match(start, e)).collect();
    println!("valid emails: {:?}", ok);
    let st = State::Split(1, 2);
    println!("{:?} {:?} {:?}", st, State::Jump(7), State::Match(Pred(Box::new(|c| c == 'q'))));
}
