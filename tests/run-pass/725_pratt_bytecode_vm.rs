// A mini language: tokenizer, Pratt expression parser with statements
// (let, while, if/else, fn, return, print), compiled to stack bytecode with
// jumps and calls, executed by a VM with call frames.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok { Num(i64), Id(String), Op(&'static str), Kw(&'static str), Eof }

fn lex(src: &str) -> Vec<Tok> {
    let ops = ["==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "<", ">", "=", "(", ")", "{", "}", ",", ";", "!"];
    let kws = ["let", "while", "if", "else", "fn", "return", "print"];
    let b = src.as_bytes();
    let mut i = 0;
    let mut out = Vec::new();
    'outer: while i < b.len() {
        let c = b[i] as char;
        if c.is_whitespace() { i += 1; continue; }
        if c == '#' { while i < b.len() && b[i] != b'\n' { i += 1; } continue; }
        if c.is_ascii_digit() {
            let s = i;
            while i < b.len() && b[i].is_ascii_digit() { i += 1; }
            out.push(Tok::Num(src[s..i].parse().unwrap()));
            continue;
        }
        if c.is_alphabetic() || c == '_' {
            let s = i;
            while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'_') { i += 1; }
            let w = &src[s..i];
            out.push(match kws.iter().find(|k| **k == w) { Some(k) => Tok::Kw(k), None => Tok::Id(w.to_string()) });
            continue;
        }
        for op in ops {
            if src[i..].starts_with(op) { out.push(Tok::Op(op)); i += op.len(); continue 'outer; }
        }
        panic!("bad char {:?}", c);
    }
    out.push(Tok::Eof);
    out
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op { Const(i64), Load(usize), Store(usize), Bin(&'static str), Not, Neg, Jmp(usize), Jz(usize), Call(usize, usize), Ret, Print, Pop }

struct Func { name: String, arity: usize, nlocals: usize, code: Vec<Op> }

impl fmt::Debug for Func {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "fn {}/{} locals={} ops={}", self.name, self.arity, self.nlocals, self.code.len()) }
}

struct Compiler { toks: Vec<Tok>, pos: usize, funcs: Vec<Func>, fnames: HashMap<String, usize>, locals: Vec<String>, code: Vec<Op> }

impl Compiler {
    fn peek(&self) -> &Tok { &self.toks[self.pos] }
    fn bump(&mut self) -> Tok { let t = self.toks[self.pos].clone(); self.pos += 1; t }
    fn expect(&mut self, op: &str) { match self.bump() { Tok::Op(o) if o == op => {}, t => panic!("expected {} got {:?}", op, t) } }
    fn local(&mut self, name: &str) -> usize {
        if let Some(i) = self.locals.iter().rposition(|l| l == name) { return i; }
        self.locals.push(name.to_string());
        self.locals.len() - 1
    }
    fn prec(op: &str) -> Option<(u8, u8)> {
        Some(match op { "||" => (1, 2), "&&" => (3, 4), "==" | "!=" => (5, 6), "<" | ">" | "<=" | ">=" => (7, 8), "+" | "-" => (9, 10), "*" | "/" | "%" => (11, 12), _ => return None })
    }
    fn expr(&mut self, min: u8) {
        match self.bump() {
            Tok::Num(n) => self.code.push(Op::Const(n)),
            Tok::Op("(") => { self.expr(0); self.expect(")"); }
            Tok::Op("-") => { self.expr(13); self.code.push(Op::Neg); }
            Tok::Op("!") => { self.expr(13); self.code.push(Op::Not); }
            Tok::Id(name) => {
                if *self.peek() == Tok::Op("(") {
                    self.bump();
                    let mut argc = 0;
                    while *self.peek() != Tok::Op(")") {
                        self.expr(0);
                        argc += 1;
                        if *self.peek() == Tok::Op(",") { self.bump(); }
                    }
                    self.bump();
                    let fi = *self.fnames.get(&name).unwrap_or_else(|| panic!("unknown fn {}", name));
                    self.code.push(Op::Call(fi, argc));
                } else {
                    let i = self.local(&name);
                    self.code.push(Op::Load(i));
                }
            }
            t => panic!("unexpected {:?}", t),
        }
        loop {
            let op = match self.peek() { Tok::Op(o) => *o, _ => break };
            let Some((l, r)) = Self::prec(op) else { break };
            if l < min { break; }
            self.bump();
            if op == "&&" || op == "||" {
                // short circuit: a && b  ==> a; jz L; b; jmp E; L: const 0
                let jz_or_skip = self.code.len();
                if op == "&&" { self.code.push(Op::Jz(0)); } else { self.code.push(Op::Not); self.code.push(Op::Jz(0)); }
                self.expr(r);
                self.code.push(Op::Not);
                self.code.push(Op::Not);
                let jend = self.code.len();
                self.code.push(Op::Jmp(0));
                let here = self.code.len();
                let jz_at = if op == "&&" { jz_or_skip } else { jz_or_skip + 1 };
                self.code[jz_at] = Op::Jz(here);
                self.code.push(Op::Const(if op == "&&" { 0 } else { 1 }));
                let end = self.code.len();
                self.code[jend] = Op::Jmp(end);
            } else {
                self.expr(r);
                self.code.push(Op::Bin(op));
            }
        }
    }
    fn block(&mut self) {
        self.expect("{");
        while *self.peek() != Tok::Op("}") { self.stmt(); }
        self.bump();
    }
    fn stmt(&mut self) {
        match self.peek().clone() {
            Tok::Kw("let") => {
                self.bump();
                let Tok::Id(n) = self.bump() else { panic!("name") };
                self.expect("=");
                self.expr(0);
                self.locals.push(n);
                let i = self.locals.len() - 1;
                self.code.push(Op::Store(i));
                self.expect(";");
            }
            Tok::Kw("print") => { self.bump(); self.expr(0); self.code.push(Op::Print); self.expect(";"); }
            Tok::Kw("return") => { self.bump(); self.expr(0); self.code.push(Op::Ret); self.expect(";"); }
            Tok::Kw("while") => {
                self.bump();
                let top = self.code.len();
                self.expr(0);
                let jz = self.code.len();
                self.code.push(Op::Jz(0));
                self.block();
                self.code.push(Op::Jmp(top));
                let end = self.code.len();
                self.code[jz] = Op::Jz(end);
            }
            Tok::Kw("if") => {
                self.bump();
                self.expr(0);
                let jz = self.code.len();
                self.code.push(Op::Jz(0));
                self.block();
                if *self.peek() == Tok::Kw("else") {
                    self.bump();
                    let j = self.code.len();
                    self.code.push(Op::Jmp(0));
                    let els = self.code.len();
                    self.code[jz] = Op::Jz(els);
                    if *self.peek() == Tok::Kw("if") { self.stmt(); } else { self.block(); }
                    let end = self.code.len();
                    self.code[j] = Op::Jmp(end);
                } else {
                    let end = self.code.len();
                    self.code[jz] = Op::Jz(end);
                }
            }
            Tok::Kw("fn") => {
                self.bump();
                let Tok::Id(name) = self.bump() else { panic!() };
                self.expect("(");
                let mut params = Vec::new();
                while let Tok::Id(p) = self.peek().clone() { self.bump(); params.push(p); if *self.peek() == Tok::Op(",") { self.bump(); } }
                self.expect(")");
                let idx = self.funcs.len();
                self.fnames.insert(name.clone(), idx);
                self.funcs.push(Func { name: name.clone(), arity: params.len(), nlocals: 0, code: vec![] });
                let saved_locals = std::mem::replace(&mut self.locals, params);
                let saved_code = std::mem::take(&mut self.code);
                self.block();
                self.code.push(Op::Const(0));
                self.code.push(Op::Ret);
                let body = std::mem::replace(&mut self.code, saved_code);
                let locals = std::mem::replace(&mut self.locals, saved_locals);
                self.funcs[idx].code = body;
                self.funcs[idx].nlocals = locals.len();
            }
            Tok::Id(n) if self.toks[self.pos + 1] == Tok::Op("=") => {
                self.bump(); self.bump();
                self.expr(0);
                let i = self.local(&n);
                self.code.push(Op::Store(i));
                self.expect(";");
            }
            _ => { self.expr(0); self.code.push(Op::Pop); self.expect(";"); }
        }
    }
}

fn run(funcs: &[Func], main: usize, out: &mut Vec<String>) -> (i64, u64) {
    struct Frame { f: usize, ip: usize, base: usize }
    let mut stack: Vec<i64> = vec![0; funcs[main].nlocals];
    let mut frames = vec![Frame { f: main, ip: 0, base: 0 }];
    let mut steps = 0u64;
    loop {
        steps += 1;
        let fr = frames.last_mut().unwrap();
        let op = funcs[fr.f].code[fr.ip];
        fr.ip += 1;
        match op {
            Op::Const(n) => stack.push(n),
            Op::Load(i) => { let v = stack[fr.base + i]; stack.push(v) }
            Op::Store(i) => { let v = stack.pop().unwrap(); stack[fr.base + i] = v }
            Op::Pop => { stack.pop(); }
            Op::Neg => { let v = stack.pop().unwrap(); stack.push(-v) }
            Op::Not => { let v = stack.pop().unwrap(); stack.push((v == 0) as i64) }
            Op::Bin(o) => {
                let b = stack.pop().unwrap();
                let a = stack.pop().unwrap();
                stack.push(match o {
                    "+" => a + b, "-" => a - b, "*" => a * b, "/" => a / b, "%" => a % b,
                    "<" => (a < b) as i64, ">" => (a > b) as i64, "<=" => (a <= b) as i64, ">=" => (a >= b) as i64,
                    "==" => (a == b) as i64, "!=" => (a != b) as i64,
                    _ => unreachable!(),
                });
            }
            Op::Jmp(t) => fr.ip = t,
            Op::Jz(t) => { if stack.pop().unwrap() == 0 { fr.ip = t } }
            Op::Print => out.push(stack.pop().unwrap().to_string()),
            Op::Call(fi, argc) => {
                assert_eq!(argc, funcs[fi].arity, "arity mismatch for {}", funcs[fi].name);
                let base = stack.len() - argc;
                stack.resize(base + funcs[fi].nlocals.max(argc), 0);
                frames.push(Frame { f: fi, ip: 0, base });
            }
            Op::Ret => {
                let v = stack.pop().unwrap();
                let fr = frames.pop().unwrap();
                if frames.is_empty() { return (v, steps); }
                stack.truncate(fr.base);
                stack.push(v);
            }
        }
    }
}

const PROGRAM: &str = "
fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); }
fn gcd(a, b) { while b != 0 { let t = b; b = a % b; a = t; } return a; }
fn is_prime(n) {
    if n < 2 { return 0; }
    let d = 2;
    while d * d <= n { if n % d == 0 { return 0; } d = d + 1; }
    return 1;
}
fn collatz(n) { let steps = 0; while n != 1 { if n % 2 == 0 { n = n / 2; } else { n = 3 * n + 1; } steps = steps + 1; } return steps; }
let i = 0;
while i < 12 { print fib(i); i = i + 1; }
print gcd(1071, 462);
let count = 0; let k = 0;
while k < 100 { if is_prime(k) { count = count + 1; } k = k + 1; }
print count;
print collatz(27);
print -(3 - 10) * 2 % 5;
print (1 < 2) && (3 > 4) || !(0);
print 0 && fib(30);  # short circuit avoids the expensive call
if count > 30 { print 1; } else if count > 20 { print 2; } else { print 3; }
return 42;
";

fn main() {
    let toks = lex(PROGRAM);
    println!("{} tokens; first five: {:?}", toks.len(), &toks[..5]);
    let mut c = Compiler { toks, pos: 0, funcs: vec![], fnames: HashMap::new(), locals: vec![], code: vec![] };
    while *c.peek() != Tok::Eof { c.stmt(); }
    let main_code = std::mem::take(&mut c.code);
    let nl = c.locals.len();
    c.funcs.push(Func { name: "<main>".into(), arity: 0, nlocals: nl, code: main_code });
    for f in &c.funcs { println!("{:?}", f); }
    println!("gcd code: {:?}", c.funcs[1].code);
    let mut out = Vec::new();
    let (ret, steps) = run(&c.funcs, c.funcs.len() - 1, &mut out);
    println!("output: {}", out.join(" "));
    println!("returned {} after {} steps", ret, steps);
}
