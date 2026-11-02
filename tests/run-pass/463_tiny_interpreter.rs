// A tiny expression-language interpreter: tokenizer, recursive-descent parser into a Box-based AST,
// environment with lexical scopes (Vec<HashMap>), let/if/while/functions with closures capturing env
// by value, and error reporting via Result.
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Tok { Num(i64), Id(String), Sym(char), Op2(&'static str), Kw(&'static str) }

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let (mut i, mut out) = (0, vec![]);
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() { i += 1; } else if c.is_ascii_digit() || c.is_alphabetic() || c == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') && (c.is_ascii_digit() == cs[i].is_ascii_digit() || !c.is_ascii_digit()) {
                i += 1;
            }
            let w: String = cs[st..i].iter().collect();
            let kw = ["let", "if", "else", "while", "fn"].iter().find(|k| **k == w);
            out.push(if c.is_ascii_digit() { Tok::Num(w.parse().unwrap()) } else if let Some(k) = kw { Tok::Kw(k) } else { Tok::Id(w) });
        } else {
            let two: String = cs[i..(i + 2).min(cs.len())].iter().collect();
            if let Some(op) = ["==", "<=", "!=", "&&"].iter().find(|o| **o == two) { out.push(Tok::Op2(op)); i += 2; }
            else if "+-*/%<>=(){};,".contains(c) { out.push(Tok::Sym(c)); i += 1; }
            else { return Err(format!("unexpected char {:?} at {}", c, i)); }
        }
    }
    Ok(out)
}

#[derive(Debug, Clone)]
enum Expr {
    Num(i64), Var(String), Bin(String, Box<Expr>, Box<Expr>), Let(String, Box<Expr>), Assign(String, Box<Expr>),
    If(Box<Expr>, Box<Expr>, Option<Box<Expr>>), While(Box<Expr>, Box<Expr>), Block(Vec<Expr>),
    Func(Vec<String>, Rc<Expr>), Call(Box<Expr>, Vec<Expr>),
}

struct Parser { t: Vec<Tok>, p: usize }

impl Parser {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.p) }
    fn eat(&mut self, t: &Tok) -> bool { let hit = self.peek() == Some(t); if hit { self.p += 1; } hit }
    fn expect(&mut self, c: char) -> Result<(), String> {
        if self.eat(&Tok::Sym(c)) { Ok(()) } else { Err(format!("expected {:?} at token {} ({:?})", c, self.p, self.peek())) }
    }
    fn block(&mut self) -> Result<Expr, String> {
        self.expect('{')?;
        let mut v = vec![];
        while !self.eat(&Tok::Sym('}')) { v.push(self.stmt()?); self.eat(&Tok::Sym(';')); }
        Ok(Expr::Block(v))
    }
    fn stmt(&mut self) -> Result<Expr, String> {
        if self.eat(&Tok::Kw("let")) {
            let Some(Tok::Id(name)) = self.t.get(self.p).cloned() else { return Err(format!("expected name, got {:?}", self.peek())) };
            self.p += 1;
            self.expect('=')?;
            return Ok(Expr::Let(name, Box::new(self.expr(0)?)));
        }
        if let (Some(Tok::Id(n)), Some(Tok::Sym('='))) = (self.t.get(self.p).cloned(), self.t.get(self.p + 1)) {
            self.p += 2;
            return Ok(Expr::Assign(n, Box::new(self.expr(0)?)));
        }
        self.expr(0)
    }
    fn prec(t: &Tok) -> Option<(u8, String)> {
        match t {
            Tok::Op2("&&") => Some((1, "&&".into())), Tok::Op2(o) => Some((2, o.to_string())),
            Tok::Sym(c @ ('<' | '>')) => Some((2, c.to_string())), Tok::Sym(c @ ('+' | '-')) => Some((3, c.to_string())),
            Tok::Sym(c @ ('*' | '/' | '%')) => Some((4, c.to_string())), _ => None,
        }
    }
    fn expr(&mut self, min: u8) -> Result<Expr, String> {
        let mut lhs = self.atom()?;
        while let Some((p, op)) = self.peek().and_then(Parser::prec) {
            if p <= min && min > 0 || p < min { break; }
            self.p += 1;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(self.expr(p)?));
        }
        Ok(lhs)
    }
    fn atom(&mut self) -> Result<Expr, String> {
        let t = self.t.get(self.p).cloned().ok_or("unexpected end")?;
        self.p += 1;
        let mut e = match t {
            Tok::Num(n) => Expr::Num(n),
            Tok::Id(n) => Expr::Var(n),
            Tok::Sym('(') => { let e = self.expr(0)?; self.expect(')')?; e }
            Tok::Sym('{') => { self.p -= 1; self.block()? }
            Tok::Kw("if") => {
                let (c, th) = (self.expr(0)?, self.block()?);
                let el = if self.eat(&Tok::Kw("else")) { Some(Box::new(if self.peek() == Some(&Tok::Kw("if")) { self.atom()? } else { self.block()? })) } else { None };
                Expr::If(Box::new(c), Box::new(th), el)
            }
            Tok::Kw("while") => Expr::While(Box::new(self.expr(0)?), Box::new(self.block()?)),
            Tok::Kw("fn") => {
                self.expect('(')?;
                let mut ps = vec![];
                while let Some(Tok::Id(n)) = self.t.get(self.p).cloned() { ps.push(n); self.p += 1; self.eat(&Tok::Sym(',')); }
                self.expect(')')?;
                Expr::Func(ps, Rc::new(self.block()?))
            }
            other => return Err(format!("unexpected token {:?}", other)),
        };
        while self.eat(&Tok::Sym('(')) {
            let mut args = vec![];
            while !self.eat(&Tok::Sym(')')) { args.push(self.expr(0)?); self.eat(&Tok::Sym(',')); }
            e = Expr::Call(Box::new(e), args);
        }
        Ok(e)
    }
}

#[derive(Clone)]
enum Val { Int(i64), Func(Vec<String>, Rc<Expr>, Vec<HashMap<String, Val>>), Unit }
impl fmt::Display for Val {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { Val::Int(n) => write!(f, "{}", n), Val::Func(ps, ..) => write!(f, "<fn/{}>", ps.len()), Val::Unit => write!(f, "()") }
    }
}

fn eval(e: &Expr, env: &mut Vec<HashMap<String, Val>>, fuel: &mut u32) -> Result<Val, String> {
    *fuel = fuel.checked_sub(1).ok_or("out of fuel")?;
    let int = |v: Val| match v { Val::Int(n) => Ok(n), other => Err(format!("expected int, got {}", other)) };
    Ok(match e {
        Expr::Num(n) => Val::Int(*n),
        Expr::Var(n) => env.iter().rev().find_map(|s| s.get(n)).cloned().ok_or(format!("unbound {}", n))?,
        Expr::Bin(op, a, b) => {
            let x = int(eval(a, env, fuel)?)?;
            if op == "&&" && x == 0 { return Ok(Val::Int(0)); }
            let y = int(eval(b, env, fuel)?)?;
            Val::Int(match op.as_str() {
                "+" => x + y, "-" => x - y, "*" => x * y,
                "/" | "%" if y == 0 => return Err("division by zero".into()),
                "/" => x / y, "%" => x % y,
                "<" => (x < y) as i64, ">" => (x > y) as i64, "<=" => (x <= y) as i64,
                "==" => (x == y) as i64, "!=" => (x != y) as i64, "&&" => (y != 0) as i64,
                _ => unreachable!(),
            })
        }
        Expr::Let(n, v) => { let v = eval(v, env, fuel)?; env.last_mut().unwrap().insert(n.clone(), v); Val::Unit }
        Expr::Assign(n, v) => { let v = eval(v, env, fuel)?; env.iter_mut().rev().find(|s| s.contains_key(n)).ok_or(format!("assign to unbound {}", n))?.insert(n.clone(), v); Val::Unit }
        Expr::If(c, t, el) => if int(eval(c, env, fuel)?)? != 0 { eval(t, env, fuel)? } else if let Some(el) = el { eval(el, env, fuel)? } else { Val::Unit },
        Expr::While(c, b) => { while int(eval(c, env, fuel)?)? != 0 { eval(b, env, fuel)?; } Val::Unit }
        Expr::Block(es) => {
            env.push(HashMap::new());
            let mut last = Val::Unit;
            for e in es { match eval(e, env, fuel) { Ok(v) => last = v, Err(er) => { env.pop(); return Err(er); } } }
            env.pop();
            last
        }
        Expr::Func(ps, body) => Val::Func(ps.clone(), body.clone(), env.clone()),
        Expr::Call(f, args) => {
            let fv = eval(f, env, fuel)?;
            let Val::Func(ps, body, mut cenv) = fv.clone() else { return Err(format!("not callable: {}", fv)) };
            if ps.len() != args.len() { return Err(format!("arity mismatch: want {} got {}", ps.len(), args.len())); }
            let mut frame = HashMap::new();
            for (p, a) in ps.iter().zip(args) { frame.insert(p.clone(), eval(a, env, fuel)?); }
            if let Expr::Var(name) = &**f { frame.entry(name.clone()).or_insert(fv.clone()); }
            cenv.push(frame);
            eval(&body, &mut cenv, fuel)?
        }
    })
}

fn run(src: &str) -> String {
    let toks = match lex(src) { Ok(t) => t, Err(e) => return format!("lex error: {}", e) };
    let prog = match (Parser { t: toks, p: 0 }).block() { Ok(b) => b, Err(e) => return format!("parse error: {}", e) };
    let (mut env, mut fuel) = (vec![HashMap::new()], 100_000);
    match eval(&prog, &mut env, &mut fuel) { Ok(v) => format!("{} (fuel left {})", v, fuel), Err(e) => format!("runtime error: {}", e) }
}

fn main() {
    let progs = [
        "{ 1 + 2 * 3 - 4 }",
        "{ let x = 10; let y = x * x; y - x }",
        "{ let s = 0; let i = 1; while i <= 10 { s = s + i * i; i = i + 1 }; s }",
        "{ let fact = fn(n) { if n < 2 { 1 } else { n * fact(n - 1) } }; fact(10) }",
        "{ let fib = fn(n) { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }; fib(15) }",
        "{ let mk = fn(k) { fn(x) { x + k } }; let add5 = mk(5); add5(37) }",
        "{ let x = 1; { let x = 2; x = 3 }; x }",
        "{ let g = 7; if g > 5 && g < 10 { 1 } else if g == 5 { 2 } else { 3 } }",
        "{ 10 / (5 - 5) }",
        "{ let f = fn(a, b) { a }; f(1) }",
        "{ y + 1 }",
        "{ let i = 0; while 1 { i = i + 1 } }",
        "{ 1 + }",
        "{ 3 $ 4 }",
        "{ let c = 0; let n = 27; while n != 1 { if n % 2 == 0 { n = n / 2 } else { n = 3 * n + 1 }; c = c + 1 }; c }",
        "{ fn(x) { x } }",
        "{ let a = 12abc; a }",
    ];
    for p in progs { println!("{} => {}", p, run(p)); }
}
