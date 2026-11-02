// A tokenizer + Pratt (top-down operator precedence) parser producing an AST, with prefix/infix/
// postfix operators, right-assoc exponent, ternary, function calls, and an evaluator with env.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Ident(String),
    Op(&'static str),
    LParen,
    RParen,
    Comma,
    Question,
    Colon,
    Eof,
}

fn lex(src: &str) -> Result<Vec<Tok>, String> {
    let cs: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    const OPS: [&str; 15] = ["**", "==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "<", ">", "!"];
    while i < cs.len() {
        let c = cs[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || (c == '.' && cs.get(i + 1).map_or(false, |d| d.is_ascii_digit())) {
            let st = i;
            while i < cs.len() && (cs[i].is_ascii_digit() || cs[i] == '.') {
                i += 1;
            }
            let s: String = cs[st..i].iter().collect();
            out.push(Tok::Num(s.parse().map_err(|_| format!("bad number {:?}", s))?));
        } else if c.is_alphabetic() || c == '_' {
            let st = i;
            while i < cs.len() && (cs[i].is_alphanumeric() || cs[i] == '_') {
                i += 1;
            }
            out.push(Tok::Ident(cs[st..i].iter().collect()));
        } else {
            match c {
                '(' => out.push(Tok::LParen),
                ')' => out.push(Tok::RParen),
                ',' => out.push(Tok::Comma),
                '?' => out.push(Tok::Question),
                ':' => out.push(Tok::Colon),
                _ => {
                    let rest: String = cs[i..cs.len().min(i + 2)].iter().collect();
                    let op = OPS.iter().find(|o| rest.starts_with(**o)).ok_or_else(|| format!("unexpected {:?} at {}", c, i))?;
                    out.push(Tok::Op(op));
                    i += op.len();
                    continue;
                }
            }
            i += 1;
        }
    }
    out.push(Tok::Eof);
    Ok(out)
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    Unary(&'static str, Box<Expr>),
    Binary(&'static str, Box<Expr>, Box<Expr>),
    Ternary(Box<Expr>, Box<Expr>, Box<Expr>),
    Call(String, Vec<Expr>),
    Factorial(Box<Expr>),
}

impl fmt::Display for Expr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Expr::Num(n) => write!(f, "{}", n),
            Expr::Var(v) => write!(f, "{}", v),
            Expr::Unary(op, e) => write!(f, "({}{})", op, e),
            Expr::Binary(op, a, b) => write!(f, "({} {} {})", a, op, b),
            Expr::Ternary(c, a, b) => write!(f, "({} ? {} : {})", c, a, b),
            Expr::Call(n, args) => {
                write!(f, "{}(", n)?;
                for (i, a) in args.iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", a)?;
                }
                write!(f, ")")
            }
            Expr::Factorial(e) => write!(f, "({}!)", e),
        }
    }
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

fn infix_bp(op: &str) -> Option<(u8, u8)> {
    Some(match op {
        "||" => (1, 2),
        "&&" => (3, 4),
        "==" | "!=" => (5, 6),
        "<" | ">" | "<=" | ">=" => (7, 8),
        "+" | "-" => (9, 10),
        "*" | "/" | "%" => (11, 12),
        "**" => (16, 15), // right associative
        _ => return None,
    })
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        self.pos += 1;
        t
    }
    fn expect(&mut self, t: Tok) -> Result<(), String> {
        let got = self.next();
        if got == t {
            Ok(())
        } else {
            Err(format!("expected {:?}, got {:?}", t, got))
        }
    }

    fn expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = match self.next() {
            Tok::Num(n) => Expr::Num(n),
            Tok::Ident(name) => {
                if *self.peek() == Tok::LParen {
                    self.next();
                    let mut args = vec![];
                    if *self.peek() != Tok::RParen {
                        loop {
                            args.push(self.expr(0)?);
                            if *self.peek() == Tok::Comma {
                                self.next();
                            } else {
                                break;
                            }
                        }
                    }
                    self.expect(Tok::RParen)?;
                    Expr::Call(name, args)
                } else {
                    Expr::Var(name)
                }
            }
            Tok::LParen => {
                let e = self.expr(0)?;
                self.expect(Tok::RParen)?;
                e
            }
            Tok::Op(op @ ("-" | "!")) => Expr::Unary(op, Box::new(self.expr(13)?)),
            t => return Err(format!("unexpected token {:?}", t)),
        };
        loop {
            match self.peek().clone() {
                Tok::Op("!") => {
                    // postfix factorial binds tightest
                    if 17 < min_bp {
                        break;
                    }
                    self.next();
                    lhs = Expr::Factorial(Box::new(lhs));
                }
                Tok::Op(op) => {
                    let Some((l, r)) = infix_bp(op) else { break };
                    if l < min_bp {
                        break;
                    }
                    self.next();
                    let rhs = self.expr(r)?;
                    lhs = Expr::Binary(op, Box::new(lhs), Box::new(rhs));
                }
                Tok::Question => {
                    if 0 < min_bp {
                        break;
                    }
                    self.next();
                    let a = self.expr(0)?;
                    self.expect(Tok::Colon)?;
                    let b = self.expr(0)?;
                    lhs = Expr::Ternary(Box::new(lhs), Box::new(a), Box::new(b));
                }
                _ => break,
            }
        }
        Ok(lhs)
    }
}

fn eval(e: &Expr, env: &HashMap<&str, f64>) -> Result<f64, String> {
    let b = |x: bool| if x { 1.0 } else { 0.0 };
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Var(v) => *env.get(v.as_str()).ok_or_else(|| format!("unbound {}", v))?,
        Expr::Unary("-", x) => -eval(x, env)?,
        Expr::Unary(_, x) => b(eval(x, env)? == 0.0),
        Expr::Factorial(x) => {
            let n = eval(x, env)?;
            (1..=n as u64).product::<u64>() as f64
        }
        Expr::Ternary(c, a, bb) => {
            if eval(c, env)? != 0.0 {
                eval(a, env)?
            } else {
                eval(bb, env)?
            }
        }
        Expr::Call(name, args) => {
            let vals = args.iter().map(|a| eval(a, env)).collect::<Result<Vec<_>, _>>()?;
            match (name.as_str(), vals.as_slice()) {
                ("max", [first, rest @ ..]) => rest.iter().fold(*first, |m, x| m.max(*x)),
                ("min", [first, rest @ ..]) => rest.iter().fold(*first, |m, x| m.min(*x)),
                ("abs", [x]) => x.abs(),
                ("sqrt", [x]) => x.sqrt(),
                (n, a) => return Err(format!("bad call {}/{}", n, a.len())),
            }
        }
        Expr::Binary(op, l, r) => {
            let (x, y) = (eval(l, env)?, eval(r, env)?);
            match *op {
                "+" => x + y,
                "-" => x - y,
                "*" => x * y,
                "/" => {
                    if y == 0.0 {
                        return Err("division by zero".into());
                    }
                    x / y
                }
                "%" => x % y,
                "**" => x.powf(y),
                "<" => b(x < y),
                ">" => b(x > y),
                "<=" => b(x <= y),
                ">=" => b(x >= y),
                "==" => b(x == y),
                "!=" => b(x != y),
                "&&" => b(x != 0.0 && y != 0.0),
                "||" => b(x != 0.0 || y != 0.0),
                _ => unreachable!(),
            }
        }
    })
}

fn run(src: &str, env: &HashMap<&str, f64>) {
    let parsed = lex(src).and_then(|toks| {
        let mut p = Parser { toks, pos: 0 };
        let e = p.expr(0)?;
        if *p.peek() != Tok::Eof {
            return Err(format!("trailing {:?}", p.peek()));
        }
        Ok(e)
    });
    match parsed {
        Ok(e) => println!("{:<30} => {:<40} = {:?}", src, e.to_string(), eval(&e, env)),
        Err(err) => println!("{:<30} => parse error: {}", src, err),
    }
}

fn main() {
    let env: HashMap<&str, f64> = [("x", 3.0), ("y", 4.0), ("pi", 3.5)].into_iter().collect();
    for src in [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "2 ** 3 ** 2",
        "-2 ** 2",
        "-x + -y * 2",
        "10 - 4 - 3",
        "100 / 10 / 5",
        "3! + 1",
        "x < y && y < 5 || 0",
        "!(x == 3) ? 1 : 2",
        "x > 2 ? y > 10 ? 1 : 2 : 3",
        "max(x, y, 2 * x) + min(1, abs(-7))",
        "sqrt(x*x + y*y)",
        "17 % 5 * 2",
        "1 / (x - 3)",
        "foo(1)",
        "z + 1",
        "1 +",
        "(1 + 2",
        "1 2",
        "3 $ 4",
    ] {
        run(src, &env);
    }
}
