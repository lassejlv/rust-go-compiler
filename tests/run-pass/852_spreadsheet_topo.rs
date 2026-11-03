use std::collections::{BTreeMap, BTreeSet, VecDeque};
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Formula { Lit(f64), Ref(String), Bin(Box<Formula>, char, Box<Formula>), Func(String, Vec<Formula>), Range(String, String) }

#[derive(Debug, Clone, PartialEq)]
enum CellVal { Num(f64), Err(String) }
impl fmt::Display for CellVal {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { CellVal::Num(n) if n.fract() == 0.0 && n.abs() < 1e15 => write!(f, "{:.0}", n), CellVal::Num(n) => write!(f, "{:.4}", n), CellVal::Err(e) => write!(f, "#{}", e) }
    }
}

struct Parser<'a> { s: &'a [u8], i: usize }
impl<'a> Parser<'a> {
    fn ws(&mut self) { while self.i < self.s.len() && self.s[self.i] == b' ' { self.i += 1; } }
    fn peek(&mut self) -> Option<u8> { self.ws(); self.s.get(self.i).copied() }
    fn expr(&mut self) -> Result<Formula, String> {
        let mut l = self.term()?;
        while let Some(c @ (b'+' | b'-')) = self.peek() { self.i += 1; let r = self.term()?; l = Formula::Bin(Box::new(l), c as char, Box::new(r)); }
        Ok(l)
    }
    fn term(&mut self) -> Result<Formula, String> {
        let mut l = self.atom()?;
        while let Some(c @ (b'*' | b'/')) = self.peek() { self.i += 1; let r = self.atom()?; l = Formula::Bin(Box::new(l), c as char, Box::new(r)); }
        Ok(l)
    }
    fn ident(&mut self) -> String { let st = self.i; while self.i < self.s.len() && self.s[self.i].is_ascii_alphanumeric() { self.i += 1; } String::from_utf8_lossy(&self.s[st..self.i]).to_uppercase() }
    fn atom(&mut self) -> Result<Formula, String> {
        match self.peek() {
            Some(b'(') => { self.i += 1; let e = self.expr()?; if self.peek() != Some(b')') { return Err("expected )".into()); } self.i += 1; Ok(e) }
            Some(c) if c.is_ascii_digit() || c == b'.' => { let st = self.i; while self.i < self.s.len() && (self.s[self.i].is_ascii_digit() || self.s[self.i] == b'.') { self.i += 1; } std::str::from_utf8(&self.s[st..self.i]).unwrap().parse().map(Formula::Lit).map_err(|e| format!("{}", e)) }
            Some(b'-') => { self.i += 1; let a = self.atom()?; Ok(Formula::Bin(Box::new(Formula::Lit(0.0)), '-', Box::new(a))) }
            Some(c) if c.is_ascii_alphabetic() => {
                let id = self.ident();
                if self.peek() == Some(b'(') {
                    self.i += 1;
                    let mut args = vec![];
                    if self.peek() != Some(b')') { loop { args.push(self.arg()?); if self.peek() == Some(b',') { self.i += 1; } else { break; } } }
                    if self.peek() != Some(b')') { return Err("expected ) after args".into()); }
                    self.i += 1;
                    Ok(Formula::Func(id, args))
                } else { Ok(Formula::Ref(id)) }
            }
            other => Err(format!("unexpected {:?}", other.map(|c| c as char))),
        }
    }
    fn arg(&mut self) -> Result<Formula, String> {
        let save = self.i;
        if let Some(c) = self.peek() { if c.is_ascii_alphabetic() { let a = self.ident(); if self.peek() == Some(b':') { self.i += 1; self.ws(); let b = self.ident(); return Ok(Formula::Range(a, b)); } } }
        self.i = save;
        self.expr()
    }
}

fn split_ref(r: &str) -> (char, u32) { (r.chars().next().unwrap(), r[1..].parse().unwrap_or(0)) }
fn expand(a: &str, b: &str) -> Vec<String> {
    let ((c1, r1), (c2, r2)) = (split_ref(a), split_ref(b));
    let mut v = vec![];
    for c in c1.min(c2)..=c1.max(c2) { for r in r1.min(r2)..=r1.max(r2) { v.push(format!("{}{}", c, r)); } }
    v
}
fn deps(f: &Formula, out: &mut BTreeSet<String>) {
    match f {
        Formula::Lit(_) => {}
        Formula::Ref(r) => { out.insert(r.clone()); }
        Formula::Bin(a, _, b) => { deps(a, out); deps(b, out); }
        Formula::Func(_, args) => args.iter().for_each(|a| deps(a, out)),
        Formula::Range(a, b) => out.extend(expand(a, b)),
    }
}
fn eval(f: &Formula, vals: &BTreeMap<String, CellVal>) -> Result<f64, String> {
    let get = |r: &str| match vals.get(r) { Some(CellVal::Num(n)) => Ok(*n), Some(CellVal::Err(e)) => Err(e.clone()), None => Ok(0.0) };
    Ok(match f {
        Formula::Lit(n) => *n,
        Formula::Ref(r) => get(r)?,
        Formula::Range(..) => return Err("VALUE".into()),
        Formula::Bin(a, op, b) => { let (x, y) = (eval(a, vals)?, eval(b, vals)?); match op { '+' => x + y, '-' => x - y, '*' => x * y, _ => if y == 0.0 { return Err("DIV/0".into()) } else { x / y } } }
        Formula::Func(name, args) => {
            let mut nums = vec![];
            for a in args { if let Formula::Range(x, y) = a { for r in expand(x, y) { nums.push(get(&r)?); } } else { nums.push(eval(a, vals)?); } }
            match name.as_str() {
                "SUM" => nums.iter().sum(),
                "AVG" => if nums.is_empty() { return Err("DIV/0".into()) } else { nums.iter().sum::<f64>() / nums.len() as f64 },
                "MAX" => nums.iter().cloned().fold(f64::NEG_INFINITY, f64::max),
                "MIN" => nums.iter().cloned().fold(f64::INFINITY, f64::min),
                "IF" => if nums.len() == 3 { if nums[0] != 0.0 { nums[1] } else { nums[2] } } else { return Err("ARGS".into()) },
                _ => return Err(format!("NAME?{}", name)),
            }
        }
    })
}

fn main() {
    let sheet = [("A1", "10"), ("A2", "20"), ("A3", "A1 + A2 * 2"), ("A4", "sum(A1:A3)"), ("B1", "A4 / (A1 - 10)"), ("B2", "avg(A1:A4, 100)"), ("B3", "max(A1:A4) - min(a1:a4)"),
        ("B4", "B5 + 1"), ("B5", "B4 + 1"), ("C1", "B1 + 1"), ("C2", "if(A1 - 10, 1, 2) * -3"), ("C3", "foo(1)"), ("C4", "(1 + 2"), ("C5", "C2 * 0.125 + D9"), ("D1", "SUM(B2:C3)")];
    let mut formulas = BTreeMap::new();
    let mut vals: BTreeMap<String, CellVal> = BTreeMap::new();
    for (k, src) in sheet {
        match (Parser { s: src.as_bytes(), i: 0 }).expr().and_then(|f| { let mut p = Parser { s: src.as_bytes(), i: 0 }; p.expr()?; if p.peek().is_some() { Err(format!("trailing at {}", p.i)) } else { Ok(f) } }) {
            Ok(f) => { formulas.insert(k.to_string(), f); }
            Err(e) => { vals.insert(k.to_string(), CellVal::Err(format!("PARSE({})", e))); }
        }
    }
    let mut indeg: BTreeMap<String, usize> = BTreeMap::new();
    let mut users: BTreeMap<String, Vec<String>> = BTreeMap::new();
    for (k, f) in &formulas {
        let mut d = BTreeSet::new(); deps(f, &mut d);
        indeg.insert(k.clone(), d.iter().filter(|x| formulas.contains_key(*x)).count());
        for x in d { users.entry(x).or_default().push(k.clone()); }
    }
    let mut q: VecDeque<String> = indeg.iter().filter(|(_, &n)| n == 0).map(|(k, _)| k.clone()).collect();
    let mut order = vec![];
    while let Some(k) = q.pop_front() {
        let v = match eval(&formulas[&k], &vals) { Ok(n) => CellVal::Num(n), Err(e) => CellVal::Err(e) };
        vals.insert(k.clone(), v);
        order.push(k.clone());
        for u in users.get(&k).cloned().unwrap_or_default() { if let Some(n) = indeg.get_mut(&u) { *n -= 1; if *n == 0 { q.push_back(u); } } }
    }
    for k in formulas.keys() { if !order.contains(k) { vals.insert(k.clone(), CellVal::Err("CYCLE".into())); } }
    println!("order: {}", order.join(" "));
    for (k, src) in sheet { println!("{:<3} {:<26} = {}", k, src, vals[k]); }
    println!("{:?}", formulas.get("A3"));
}
