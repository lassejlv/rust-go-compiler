// tokenizer + Pratt parser + evaluator with variables, functions, errors, and pretty printer
use std::collections::BTreeMap;
use std::fmt;
use std::iter::Peekable;
use std::str::Chars;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Id(String),
    Op(char),
    LP,
    RP,
    Comma,
}
struct Lexer<'a> {
    it: Peekable<Chars<'a>>,
}
impl<'a> Iterator for Lexer<'a> {
    type Item = Result<Tok, String>;
    fn next(&mut self) -> Option<Self::Item> {
        while self.it.next_if(|c| c.is_whitespace()).is_some() {}
        let c = self.it.next()?;
        Some(Ok(match c {
            '0'..='9' | '.' => {
                let mut s = c.to_string();
                while let Some(d) = self.it.next_if(|d| d.is_ascii_digit() || *d == '.') {
                    s.push(d);
                }
                match s.parse() {
                    Ok(v) => Tok::Num(v),
                    Err(_) => return Some(Err(format!("bad number {}", s))),
                }
            }
            c if c.is_alphabetic() => {
                let mut s = c.to_string();
                while let Some(d) = self.it.next_if(|d| d.is_alphanumeric()) {
                    s.push(d);
                }
                Tok::Id(s)
            }
            '+' | '-' | '*' | '/' | '^' | '=' => Tok::Op(c),
            '(' => Tok::LP,
            ')' => Tok::RP,
            ',' => Tok::Comma,
            other => return Some(Err(format!("unexpected {:?}", other))),
        }))
    }
}
#[derive(Debug, Clone)]
enum Ast {
    Num(f64),
    Var(String),
    Neg(Box<Ast>),
    Bin(char, Box<Ast>, Box<Ast>),
    Call(String, Vec<Ast>),
    Assign(String, Box<Ast>),
}
impl fmt::Display for Ast {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ast::Num(n) => write!(f, "{}", n),
            Ast::Var(v) => write!(f, "{}", v),
            Ast::Neg(e) => write!(f, "(-{})", e),
            Ast::Bin(op, a, b) => write!(f, "({} {} {})", a, op, b),
            Ast::Call(n, args) => {
                let parts: Vec<String> = args.iter().map(|a| a.to_string()).collect();
                write!(f, "{}({})", n, parts.join(", "))
            }
            Ast::Assign(n, e) => write!(f, "{} := {}", n, e),
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}
impl Parser {
    fn peek(&self) -> Option<&Tok> { self.toks.get(self.pos) }
    fn bump(&mut self) -> Option<Tok> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn bp(op: char) -> Option<(u8, u8)> {
        Some(match op { '=' => (2, 1), '+' | '-' => (3, 4), '*' | '/' => (5, 6), '^' => (8, 7), _ => return None })
    }
    fn expr(&mut self, min: u8) -> Result<Ast, String> {
        let mut lhs = match self.bump().ok_or("unexpected end")? {
            Tok::Num(n) => Ast::Num(n),
            Tok::Id(name) => {
                if self.peek() == Some(&Tok::LP) {
                    self.bump();
                    let mut args = vec![];
                    if self.peek() != Some(&Tok::RP) {
                        loop {
                            args.push(self.expr(0)?);
                            match self.bump() {
                                Some(Tok::Comma) => continue,
                                Some(Tok::RP) => break,
                                t => return Err(format!("expected , or ) got {:?}", t)),
                            }
                        }
                    } else {
                        self.bump();
                    }
                    Ast::Call(name, args)
                } else {
                    Ast::Var(name)
                }
            }
            Tok::Op('-') => Ast::Neg(Box::new(self.expr(7)?)),
            Tok::LP => {
                let e = self.expr(0)?;
                match self.bump() {
                    Some(Tok::RP) => e,
                    t => return Err(format!("expected ) got {:?}", t)),
                }
            }
            t => return Err(format!("unexpected token {:?}", t)),
        };
        while let Some(Tok::Op(op)) = self.peek().cloned() {
            let (l, r) = Self::bp(op).ok_or("bad op")?;
            if l < min {
                break;
            }
            self.bump();
            let rhs = self.expr(r)?;
            lhs = if op == '=' {
                match lhs {
                    Ast::Var(n) => Ast::Assign(n, Box::new(rhs)),
                    _ => return Err("cannot assign".into()),
                }
            } else {
                Ast::Bin(op, Box::new(lhs), Box::new(rhs))
            };
        }
        Ok(lhs)
    }
}

fn eval(a: &Ast, env: &mut BTreeMap<String, f64>) -> Result<f64, String> {
    Ok(match a {
        Ast::Num(n) => *n,
        Ast::Var(v) => *env.get(v).ok_or_else(|| format!("undefined {}", v))?,
        Ast::Neg(e) => -eval(e, env)?,
        Ast::Assign(n, e) => {
            let v = eval(e, env)?;
            env.insert(n.clone(), v);
            v
        }
        Ast::Bin(op, l, r) => {
            let (x, y) = (eval(l, env)?, eval(r, env)?);
            match op {
                '+' => x + y, '-' => x - y, '*' => x * y, '^' => x.powf(y),
                '/' if y == 0.0 => return Err("division by zero".into()),
                '/' => x / y,
                _ => unreachable!(),
            }
        }
        Ast::Call(n, args) => {
            let vs = args.iter().map(|a| eval(a, env)).collect::<Result<Vec<_>, _>>()?;
            match (n.as_str(), vs.as_slice()) {
                ("max", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.max(*v)),
                ("min", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.min(*v)),
                ("sqrt", [x]) => x.sqrt(),
                ("sum", vs) => vs.iter().sum(),
                (name, vs) => return Err(format!("bad call {}/{}", name, vs.len())),
            }
        }
    })
}

fn run(src: &str, env: &mut BTreeMap<String, f64>) -> Result<(String, f64), String> {
    let toks = Lexer { it: src.chars().peekable() }.collect::<Result<Vec<_>, _>>()?;
    let mut p = Parser { toks, pos: 0 };
    let ast = p.expr(0)?;
    if p.pos < p.toks.len() {
        return Err(format!("trailing tokens at {}", p.pos));
    }
    let v = eval(&ast, env)?;
    Ok((ast.to_string(), v))
}

fn main() {
    let mut env = BTreeMap::new();
    env.insert("pi".to_string(), 3.0);
    for src in [
        "1 + 2 * 3", "(1 + 2) * 3", "2 ^ 3 ^ 2", "-2 ^ 2", "x = y = 4", "x * y - 1",
        "max(1, x, 3) + min(9, 2)", "sqrt(16) / 0", "sum() + sum(1,2,3)", "z + 1",
        "foo(1)", "3 $ 4", "(1 + 2", "1 2", "r = 0.5 * pi * 2",
    ] {
        match run(src, &mut env) {
            Ok((ast, v)) => println!("{:<28} => {:<34} = {}", src, ast, v),
            Err(e) => println!("{:<28} => error: {}", src, e),
        }
    }
    println!("{:?}", env);
}
