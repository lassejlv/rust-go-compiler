// Text tokenizer + Pratt parser for arithmetic with unary minus, power (right-assoc), function
// calls (min/max/abs with varargs), variables from a map, error positions, and a printer that
// re-emits minimal parentheses. Uses lifetimes on tokens borrowing from the source string.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Tok<'s> {
    Num(f64),
    Ident(&'s str),
    Op(char),
    LParen,
    RParen,
    Comma,
}

fn tokenize(src: &str) -> Result<Vec<(usize, Tok<'_>)>, String> {
    let mut out = Vec::new();
    let mut it = src.char_indices().peekable();
    while let Some(&(i, c)) = it.peek() {
        match c {
            ' ' | '\t' => { it.next(); }
            '0'..='9' | '.' => {
                let mut end = i;
                while let Some(&(j, d)) = it.peek() {
                    if d.is_ascii_digit() || d == '.' { end = j + 1; it.next(); } else { break; }
                }
                let n = src[i..end].parse::<f64>().map_err(|_| format!("bad number {:?} at {}", &src[i..end], i))?;
                out.push((i, Tok::Num(n)));
            }
            c if c.is_alphabetic() => {
                let mut end = i;
                while let Some(&(j, d)) = it.peek() {
                    if d.is_alphanumeric() || d == '_' { end = j + d.len_utf8(); it.next(); } else { break; }
                }
                out.push((i, Tok::Ident(&src[i..end])));
            }
            '+' | '-' | '*' | '/' | '^' => { out.push((i, Tok::Op(c))); it.next(); }
            '(' => { out.push((i, Tok::LParen)); it.next(); }
            ')' => { out.push((i, Tok::RParen)); it.next(); }
            ',' => { out.push((i, Tok::Comma)); it.next(); }
            other => return Err(format!("unexpected {:?} at {}", other, i)),
        }
    }
    Ok(out)
}

#[derive(Debug)]
enum Ast<'s> {
    Num(f64),
    Var(&'s str),
    Neg(Box<Ast<'s>>),
    Bin(char, Box<Ast<'s>>, Box<Ast<'s>>),
    Call(&'s str, Vec<Ast<'s>>),
}

fn bp(op: char) -> (u8, u8) {
    match op {
        '+' | '-' => (1, 2),
        '*' | '/' => (3, 4),
        '^' => (6, 5),
        _ => (0, 0),
    }
}

struct Parser<'s> {
    toks: Vec<(usize, Tok<'s>)>,
    pos: usize,
    len: usize,
}

impl<'s> Parser<'s> {
    fn next(&mut self) -> Option<(usize, Tok<'s>)> {
        let t = self.toks.get(self.pos).copied();
        self.pos += 1;
        t
    }
    fn peek(&self) -> Option<Tok<'s>> {
        self.toks.get(self.pos).map(|t| t.1)
    }
    fn expr(&mut self, min: u8) -> Result<Ast<'s>, String> {
        let mut lhs = match self.next() {
            Some((_, Tok::Num(n))) => Ast::Num(n),
            Some((_, Tok::Ident(name))) if self.peek() == Some(Tok::LParen) => {
                self.pos += 1;
                let mut args = vec![];
                if self.peek() != Some(Tok::RParen) {
                    loop {
                        args.push(self.expr(0)?);
                        match self.next() {
                            Some((_, Tok::Comma)) => continue,
                            Some((_, Tok::RParen)) => break,
                            Some((p, t)) => return Err(format!("expected , or ) at {} got {:?}", p, t)),
                            None => return Err("unclosed call".into()),
                        }
                    }
                } else {
                    self.pos += 1;
                }
                Ast::Call(name, args)
            }
            Some((_, Tok::Ident(name))) => Ast::Var(name),
            Some((_, Tok::Op('-'))) => Ast::Neg(Box::new(self.expr(5)?)),
            Some((_, Tok::LParen)) => {
                let e = self.expr(0)?;
                match self.next() {
                    Some((_, Tok::RParen)) => e,
                    _ => return Err("expected )".into()),
                }
            }
            Some((p, t)) => return Err(format!("unexpected {:?} at {}", t, p)),
            None => return Err(format!("unexpected end at {}", self.len)),
        };
        while let Some(Tok::Op(op)) = self.peek() {
            let (l, r) = bp(op);
            if l < min { break; }
            self.pos += 1;
            lhs = Ast::Bin(op, Box::new(lhs), Box::new(self.expr(r)?));
        }
        Ok(lhs)
    }
}

fn eval(a: &Ast, vars: &HashMap<&str, f64>) -> Result<f64, String> {
    Ok(match a {
        Ast::Num(n) => *n,
        Ast::Var(v) => *vars.get(v).ok_or(format!("unknown variable {}", v))?,
        Ast::Neg(e) => -eval(e, vars)?,
        Ast::Bin(op, l, r) => {
            let (x, y) = (eval(l, vars)?, eval(r, vars)?);
            match op { '+' => x + y, '-' => x - y, '*' => x * y, '/' => x / y, '^' => x.powf(y), _ => unreachable!() }
        }
        Ast::Call(f, args) => {
            let vals = args.iter().map(|a| eval(a, vars)).collect::<Result<Vec<f64>, String>>()?;
            match (*f, vals.as_slice()) {
                ("abs", [x]) => x.abs(),
                ("min", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.min(*v)),
                ("max", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.max(*v)),
                (name, vs) => return Err(format!("bad call {}/{}", name, vs.len())),
            }
        }
    })
}

impl fmt::Display for Ast<'_> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        fn go(a: &Ast, parent: u8, out: &mut fmt::Formatter) -> fmt::Result {
            match a {
                Ast::Num(n) => write!(out, "{}", n),
                Ast::Var(v) => write!(out, "{}", v),
                Ast::Neg(e) => { write!(out, "-")?; go(e, 5, out) }
                Ast::Call(n, args) => {
                    write!(out, "{}(", n)?;
                    for (i, a) in args.iter().enumerate() {
                        if i > 0 { write!(out, ", ")?; }
                        go(a, 0, out)?;
                    }
                    write!(out, ")")
                }
                Ast::Bin(op, l, r) => {
                    let (lb, rb) = bp(*op);
                    let paren = lb < parent;
                    if paren { write!(out, "(")?; }
                    go(l, lb, out)?;
                    write!(out, " {} ", op)?;
                    go(r, rb, out)?;
                    if paren { write!(out, ")")?; }
                    Ok(())
                }
            }
        }
        go(self, 0, f)
    }
}

fn main() {
    let vars: HashMap<&str, f64> = [("x", 3.0), ("y", -2.0), ("π", std::f64::consts::PI)].into_iter().collect();
    let inputs = [
        "1 + 2 * 3", "(1 + 2) * 3", "2 ^ 3 ^ 2", "(2 ^ 3) ^ 2", "-x ^ 2", "x - y - 1", "x - (y - 1)",
        "max(1, x, y * -4, 2)", "min(x)", "abs(y) + abs(-7.5)", "2 * π", "10 / 4 / 5", "1 +", "foo(1, 2)", "z * 2",
        "max()", "3 $ 4", "(1 + 2", "1.2.3", "max(1 2)",
    ];
    for src in inputs {
        let res = tokenize(src).and_then(|toks| {
            let len = src.len();
            let mut p = Parser { toks, pos: 0, len };
            let ast = p.expr(0)?;
            if p.pos < p.toks.len() {
                return Err(format!("trailing input at {}", p.toks[p.pos].0));
            }
            let v = eval(&ast, &vars)?;
            Ok(format!("{} = {}", ast, v))
        });
        match res {
            Ok(s) => println!("{:<22} => {}", src, s),
            Err(e) => println!("{:<22} => error: {}", src, e),
        }
    }
}
