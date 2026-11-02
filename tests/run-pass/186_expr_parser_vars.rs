use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Op(char),
    LParen,
    RParen,
    Assign,
    Comma,
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Var(String),
    Unary(char, Box<Expr>),
    Binary(Box<Expr>, char, Box<Expr>),
    Call(String, Vec<Expr>),
}

enum Stmt {
    Let(String, Expr),
    Print(Expr),
}

#[derive(Debug)]
enum CalcError {
    Lex(char, usize),
    Parse(String),
    UnknownVar(String),
    UnknownFn(String),
    DivZero,
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CalcError::Lex(c, p) => write!(f, "unexpected char '{}' at {}", c, p),
            CalcError::Parse(m) => write!(f, "parse error: {}", m),
            CalcError::UnknownVar(v) => write!(f, "unknown variable '{}'", v),
            CalcError::UnknownFn(v) => write!(f, "unknown function '{}'", v),
            CalcError::DivZero => write!(f, "division by zero"),
        }
    }
}

fn lex(src: &str) -> Result<Vec<Token>, CalcError> {
    let chars: Vec<char> = src.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
        } else if c.is_ascii_digit() || c == '.' {
            let start = i;
            while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                i += 1;
            }
            let s: String = chars[start..i].iter().collect();
            out.push(Token::Num(s.parse().map_err(|_| CalcError::Parse(format!("bad number {}", s)))?));
        } else if c.is_alphabetic() || c == '_' {
            let start = i;
            while i < chars.len() && (chars[i].is_alphanumeric() || chars[i] == '_') {
                i += 1;
            }
            out.push(Token::Ident(chars[start..i].iter().collect()));
        } else {
            out.push(match c {
                '+' | '-' | '*' | '/' | '^' | '%' => Token::Op(c),
                '(' => Token::LParen,
                ')' => Token::RParen,
                '=' => Token::Assign,
                ',' => Token::Comma,
                _ => return Err(CalcError::Lex(c, i)),
            });
            i += 1;
        }
    }
    Ok(out)
}

struct Parser {
    toks: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    fn expect(&mut self, t: Token) -> Result<(), CalcError> {
        match self.next() {
            Some(ref x) if *x == t => Ok(()),
            other => Err(CalcError::Parse(format!("expected {:?}, got {:?}", t, other))),
        }
    }
    fn stmt(&mut self) -> Result<Stmt, CalcError> {
        if let (Some(Token::Ident(name)), Some(Token::Assign)) = (self.toks.get(0).cloned(), self.toks.get(1)) {
            if name == "let" {
                return Err(CalcError::Parse("reserved".into()));
            }
            self.pos = 2;
            let e = self.expr()?;
            return Ok(Stmt::Let(name, e));
        }
        Ok(Stmt::Print(self.expr()?))
    }
    fn expr(&mut self) -> Result<Expr, CalcError> {
        let mut lhs = self.term()?;
        while let Some(Token::Op(c @ ('+' | '-'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.term()?;
            lhs = Expr::Binary(Box::new(lhs), c, Box::new(rhs));
        }
        Ok(lhs)
    }
    fn term(&mut self) -> Result<Expr, CalcError> {
        let mut lhs = self.factor()?;
        while let Some(Token::Op(c @ ('*' | '/' | '%'))) = self.peek().cloned() {
            self.pos += 1;
            let rhs = self.factor()?;
            lhs = Expr::Binary(Box::new(lhs), c, Box::new(rhs));
        }
        Ok(lhs)
    }
    fn factor(&mut self) -> Result<Expr, CalcError> {
        let base = self.unary()?;
        if let Some(Token::Op('^')) = self.peek() {
            self.pos += 1;
            let exp = self.factor()?; // right associative
            return Ok(Expr::Binary(Box::new(base), '^', Box::new(exp)));
        }
        Ok(base)
    }
    fn unary(&mut self) -> Result<Expr, CalcError> {
        if let Some(Token::Op('-')) = self.peek() {
            self.pos += 1;
            return Ok(Expr::Unary('-', Box::new(self.unary()?)));
        }
        self.primary()
    }
    fn primary(&mut self) -> Result<Expr, CalcError> {
        match self.next() {
            Some(Token::Num(n)) => Ok(Expr::Num(n)),
            Some(Token::Ident(name)) => {
                if let Some(Token::LParen) = self.peek() {
                    self.pos += 1;
                    let mut args = Vec::new();
                    if let Some(Token::RParen) = self.peek() {
                        self.pos += 1;
                    } else {
                        loop {
                            args.push(self.expr()?);
                            match self.next() {
                                Some(Token::Comma) => continue,
                                Some(Token::RParen) => break,
                                t => return Err(CalcError::Parse(format!("bad arg list near {:?}", t))),
                            }
                        }
                    }
                    Ok(Expr::Call(name, args))
                } else {
                    Ok(Expr::Var(name))
                }
            }
            Some(Token::LParen) => {
                let e = self.expr()?;
                self.expect(Token::RParen)?;
                Ok(e)
            }
            t => Err(CalcError::Parse(format!("unexpected {:?}", t))),
        }
    }
}

fn eval(e: &Expr, env: &HashMap<String, f64>) -> Result<f64, CalcError> {
    Ok(match e {
        Expr::Num(n) => *n,
        Expr::Var(v) => *env.get(v).ok_or_else(|| CalcError::UnknownVar(v.clone()))?,
        Expr::Unary(_, x) => -eval(x, env)?,
        Expr::Binary(a, op, b) => {
            let (x, y) = (eval(a, env)?, eval(b, env)?);
            match op {
                '+' => x + y,
                '-' => x - y,
                '*' => x * y,
                '/' if y == 0.0 => return Err(CalcError::DivZero),
                '/' => x / y,
                '%' => x % y,
                '^' => x.powf(y),
                _ => unreachable!(),
            }
        }
        Expr::Call(name, args) => {
            let vals: Result<Vec<f64>, _> = args.iter().map(|a| eval(a, env)).collect();
            let vals = vals?;
            match (name.as_str(), vals.as_slice()) {
                ("max", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.max(*v)),
                ("min", [first, rest @ ..]) => rest.iter().fold(*first, |m, v| m.min(*v)),
                ("sqrt", [x]) => x.sqrt(),
                ("abs", [x]) => x.abs(),
                ("sum", vs) => vs.iter().sum(),
                _ => return Err(CalcError::UnknownFn(format!("{}/{}", name, vals.len()))),
            }
        }
    })
}

fn run_line(line: &str, env: &mut HashMap<String, f64>) -> Result<String, CalcError> {
    let toks = lex(line)?;
    let mut p = Parser { toks, pos: 0 };
    let stmt = p.stmt()?;
    if p.pos < p.toks.len() {
        return Err(CalcError::Parse(format!("trailing tokens from {}", p.pos)));
    }
    match stmt {
        Stmt::Let(name, e) => {
            let v = eval(&e, env)?;
            env.insert(name.clone(), v);
            Ok(format!("{} = {}", name, v))
        }
        Stmt::Print(e) => Ok(format!("{}", eval(&e, env)?)),
    }
}

fn main() {
    let program = [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "2 ^ 3 ^ 2",
        "-2 ^ 2",
        "x = 10",
        "y = x / 4",
        "x * y - 1",
        "r = sqrt(x * x + y * y)",
        "max(1, x, y, 7) + min(3, -y)",
        "sum() + sum(1,2,3) + abs(-0.5)",
        "z + 1",
        "10 / (x - 10)",
        "foo(1)",
        "3 $ 4",
        "(1 + 2",
        "1 2",
        "17 % 5 + 0.25",
    ];
    let mut env = HashMap::new();
    for line in program.iter() {
        match run_line(line, &mut env) {
            Ok(s) => println!("{:<32} => {}", line, s),
            Err(e) => println!("{:<32} !! {}", line, e),
        }
    }
    let mut names: Vec<_> = env.iter().collect();
    names.sort_by(|a, b| a.0.cmp(b.0));
    println!("{:?}", names);
}
