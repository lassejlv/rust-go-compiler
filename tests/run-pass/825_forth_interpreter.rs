use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Word { Lit(i64), Prim(&'static str), Call(String), If(Vec<Word>, Vec<Word>), Loop(Vec<Word>) }

#[derive(Debug)]
enum ForthErr { Underflow(&'static str), Unknown(String), DivZero, Syntax(String) }
impl fmt::Display for ForthErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ForthErr::Underflow(w) => write!(f, "stack underflow in '{}'", w),
            ForthErr::Unknown(w) => write!(f, "unknown word '{}'", w),
            ForthErr::DivZero => write!(f, "division by zero"),
            ForthErr::Syntax(s) => write!(f, "syntax: {}", s),
        }
    }
}

const PRIMS: [&str; 14] = ["+", "-", "*", "/", "mod", "dup", "drop", "swap", "over", "rot", ".", "=", "<", "emit"];

struct Forth { stack: Vec<i64>, dict: HashMap<String, Vec<Word>>, out: String, depth: usize }

impl Forth {
    fn new() -> Self { Forth { stack: vec![], dict: HashMap::new(), out: String::new(), depth: 0 } }
    fn pop(&mut self, w: &'static str) -> Result<i64, ForthErr> { self.stack.pop().ok_or(ForthErr::Underflow(w)) }
    fn parse(&self, toks: &mut std::iter::Peekable<std::vec::IntoIter<String>>, until: &[&str]) -> Result<(Vec<Word>, Option<String>), ForthErr> {
        let mut out = vec![];
        while let Some(t) = toks.next() {
            if until.contains(&t.as_str()) { return Ok((out, Some(t))); }
            let w = match t.as_str() {
                "if" => {
                    let (a, end) = self.parse(toks, &["else", "then"])?;
                    let b = if end.as_deref() == Some("else") { self.parse(toks, &["then"])?.0 } else if end.is_none() { return Err(ForthErr::Syntax("unterminated if".into())); } else { vec![] };
                    Word::If(a, b)
                }
                "begin" => { let (body, end) = self.parse(toks, &["until"])?; if end.is_none() { return Err(ForthErr::Syntax("unterminated begin".into())); } Word::Loop(body) }
                _ => {
                    if let Ok(n) = t.parse::<i64>() { Word::Lit(n) }
                    else if let Some(p) = PRIMS.iter().find(|p| **p == t) { Word::Prim(p) }
                    else { Word::Call(t) }
                }
            };
            out.push(w);
        }
        Ok((out, None))
    }
    fn exec(&mut self, words: &[Word]) -> Result<(), ForthErr> {
        self.depth += 1;
        if self.depth > 50 { return Err(ForthErr::Syntax("recursion too deep".into())); }
        for w in words {
            match w {
                Word::Lit(n) => self.stack.push(*n),
                Word::Prim(p) => self.prim(p)?,
                Word::Call(name) => {
                    let body = self.dict.get(name).cloned().ok_or_else(|| ForthErr::Unknown(name.clone()))?;
                    self.exec(&body)?;
                }
                Word::If(a, b) => { let c = self.pop("if")?; if c != 0 { self.exec(a)? } else { self.exec(b)? } }
                Word::Loop(body) => loop { self.exec(body)?; if self.pop("until")? != 0 { break; } },
            }
        }
        self.depth -= 1;
        Ok(())
    }
    fn prim(&mut self, p: &'static str) -> Result<(), ForthErr> {
        match p {
            "dup" => { let a = self.pop(p)?; self.stack.extend([a, a]); }
            "drop" => { self.pop(p)?; }
            "swap" => { let b = self.pop(p)?; let a = self.pop(p)?; self.stack.extend([b, a]); }
            "over" => { let b = self.pop(p)?; let a = self.pop(p)?; self.stack.extend([a, b, a]); }
            "rot" => { let c = self.pop(p)?; let b = self.pop(p)?; let a = self.pop(p)?; self.stack.extend([b, c, a]); }
            "." => { let a = self.pop(p)?; self.out += &format!("{} ", a); }
            "emit" => { let a = self.pop(p)?; self.out.push(char::from_u32(a as u32).unwrap_or('?')); }
            _ => {
                let b = self.pop(p)?; let a = self.pop(p)?;
                let r = match p {
                    "+" => a.wrapping_add(b), "-" => a - b, "*" => a.wrapping_mul(b),
                    "/" => { if b == 0 { return Err(ForthErr::DivZero); } a.div_euclid(b) }
                    "mod" => { if b == 0 { return Err(ForthErr::DivZero); } a.rem_euclid(b) }
                    "=" => -((a == b) as i64), "<" => -((a < b) as i64),
                    _ => unreachable!(),
                };
                self.stack.push(r);
            }
        }
        Ok(())
    }
    fn eval(&mut self, src: &str) -> Result<(), ForthErr> {
        let toks: Vec<String> = src.split_whitespace().map(|s| s.to_lowercase()).collect();
        let mut it = toks.into_iter().peekable();
        while it.peek().is_some() {
            if it.peek().map(|s| s == ":").unwrap_or(false) {
                it.next();
                let name = it.next().ok_or(ForthErr::Syntax("missing name".into()))?;
                let (body, end) = self.parse(&mut it, &[";"])?;
                if end.is_none() { return Err(ForthErr::Syntax("missing ;".into())); }
                self.dict.insert(name, body);
            } else {
                let mut chunk = vec![];
                while let Some(t) = it.peek() { if t == ":" { break; } chunk.push(it.next().unwrap()); }
                let (words, _) = self.parse(&mut chunk.into_iter().peekable(), &[])?;
                self.depth = 0;
                self.exec(&words)?;
            }
        }
        Ok(())
    }
}

fn main() {
    let programs = [
        "1 2 + 3 * .",
        ": sq dup * ; : cube dup sq * ; 3 sq . 4 cube . -2 cube .",
        ": fact dup 1 < if drop 1 else dup 1 - fact * then ; 10 fact . 20 fact .",
        ": countdown begin dup . 1 - dup 0 = until drop ; 5 countdown",
        ": fib 0 1 rot begin rot rot swap over + rot 1 - dup 0 = until drop drop ; 30 fib .",
        "72 emit 105 emit 33 emit 10 7 mod . -7 3 / . -7 3 mod .",
        "1 2 3 rot . . . 5 6 over . . .",
        "1 +",
        "5 0 /",
        "foo",
        ": broken 1 if 2",
        ": inf inf ; inf",
        ": gcd begin swap over mod dup 0 = until drop ; 1071 462 gcd . 17 5 gcd .",
        ": max2 over over < if swap then drop ; 3 9 max2 . 9 3 max2 . 4 4 = . 3 4 < .",
    ];
    for p in programs.iter() {
        let mut f = Forth::new();
        let r = f.eval(p);
        match r {
            Ok(()) => println!("ok   | {:<40} | out=[{}] stack={:?}", p.chars().take(40).collect::<String>(), f.out.trim_end(), f.stack),
            Err(e) => println!("err  | {:<40} | {} (out=[{}] stack={:?})", p.chars().take(40).collect::<String>(), e, f.out.trim_end(), f.stack),
        }
    }
    let mut f = Forth::new();
    f.eval(": a 1 ; : b a a + ; : a 100 ;").unwrap();
    f.eval("b . a .").unwrap();
    let mut names: Vec<_> = f.dict.keys().cloned().collect();
    names.sort();
    println!("{} names={:?} def(b)={:?}", f.out, names, f.dict["b"]);
}
