use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Token {
    Num(f64),
    Ident(String),
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    LParen,
    RParen,
}

#[derive(Debug)]
enum CalcError {
    UnexpectedChar(char, usize),
    UnexpectedEnd,
    UnexpectedToken(Token),
    DivByZero,
    UnknownVar(String),
}

impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CalcError::UnexpectedChar(c, p) => write!(f, "unexpected '{}' at {}", c, p),
            CalcError::UnexpectedEnd => write!(f, "unexpected end of input"),
            CalcError::UnexpectedToken(t) => write!(f, "unexpected token {:?}", t),
            CalcError::DivByZero => write!(f, "division by zero"),
            CalcError::UnknownVar(v) => write!(f, "unknown variable '{}'", v),
        }
    }
}

fn tokenize(s: &str) -> Result<Vec<Token>, CalcError> {
    let chars: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = Vec::new();
    while i < chars.len() {
        let c = chars[i];
        match c {
            ' ' => i += 1,
            '0'..='9' | '.' => {
                let start = i;
                while i < chars.len() && (chars[i].is_ascii_digit() || chars[i] == '.') {
                    i += 1;
                }
                let text: String = chars[start..i].iter().collect();
                out.push(Token::Num(text.parse().unwrap()));
            }
            'a'..='z' | 'A'..='Z' => {
                let start = i;
                while i < chars.len() && chars[i].is_alphanumeric() {
                    i += 1;
                }
                out.push(Token::Ident(chars[start..i].iter().collect()));
            }
            '+' => { out.push(Token::Plus); i += 1; }
            '-' => { out.push(Token::Minus); i += 1; }
            '*' => { out.push(Token::Star); i += 1; }
            '/' => { out.push(Token::Slash); i += 1; }
            '^' => { out.push(Token::Caret); i += 1; }
            '(' => { out.push(Token::LParen); i += 1; }
            ')' => { out.push(Token::RParen); i += 1; }
            other => return Err(CalcError::UnexpectedChar(other, i)),
        }
    }
    Ok(out)
}

struct Parser<'a> {
    toks: &'a [Token],
    pos: usize,
    vars: &'a [(&'a str, f64)],
}

impl<'a> Parser<'a> {
    fn peek(&self) -> Option<&Token> {
        self.toks.get(self.pos)
    }
    fn next(&mut self) -> Option<Token> {
        let t = self.toks.get(self.pos).cloned();
        self.pos += 1;
        t
    }
    // expr := term (('+'|'-') term)*
    fn expr(&mut self) -> Result<f64, CalcError> {
        let mut v = self.term()?;
        while let Some(t) = self.peek() {
            match t {
                Token::Plus => { self.pos += 1; v += self.term()?; }
                Token::Minus => { self.pos += 1; v -= self.term()?; }
                _ => break,
            }
        }
        Ok(v)
    }
    // term := power (('*'|'/') power)*
    fn term(&mut self) -> Result<f64, CalcError> {
        let mut v = self.power()?;
        loop {
            match self.peek() {
                Some(Token::Star) => { self.pos += 1; v *= self.power()?; }
                Some(Token::Slash) => {
                    self.pos += 1;
                    let d = self.power()?;
                    if d == 0.0 {
                        return Err(CalcError::DivByZero);
                    }
                    v /= d;
                }
                _ => return Ok(v),
            }
        }
    }
    // power := unary ('^' power)?
    fn power(&mut self) -> Result<f64, CalcError> {
        let base = self.unary()?;
        if let Some(Token::Caret) = self.peek() {
            self.pos += 1;
            let e = self.power()?;
            return Ok(base.powf(e));
        }
        Ok(base)
    }
    fn unary(&mut self) -> Result<f64, CalcError> {
        if let Some(Token::Minus) = self.peek() {
            self.pos += 1;
            return Ok(-self.unary()?);
        }
        self.atom()
    }
    fn atom(&mut self) -> Result<f64, CalcError> {
        match self.next() {
            Some(Token::Num(n)) => Ok(n),
            Some(Token::Ident(name)) => self
                .vars
                .iter()
                .find(|(k, _)| *k == name)
                .map(|(_, v)| *v)
                .ok_or(CalcError::UnknownVar(name)),
            Some(Token::LParen) => {
                let v = self.expr()?;
                match self.next() {
                    Some(Token::RParen) => Ok(v),
                    Some(t) => Err(CalcError::UnexpectedToken(t)),
                    None => Err(CalcError::UnexpectedEnd),
                }
            }
            Some(t) => Err(CalcError::UnexpectedToken(t)),
            None => Err(CalcError::UnexpectedEnd),
        }
    }
}

fn eval(s: &str, vars: &[(&str, f64)]) -> Result<f64, CalcError> {
    let toks = tokenize(s)?;
    let mut p = Parser { toks: &toks, pos: 0, vars };
    let v = p.expr()?;
    if let Some(t) = p.next() {
        return Err(CalcError::UnexpectedToken(t));
    }
    Ok(v)
}

fn main() {
    let vars = [("x", 3.0), ("pi", 3.14159), ("rate", 0.05)];
    let exprs = [
        "1 + 2 * 3",
        "(1 + 2) * 3",
        "2 ^ 3 ^ 2",
        "-2 ^ 2",
        "10 / 4 - 1.5",
        "x * x + 2 * x + 1",
        "2 * pi * x",
        "1000 * (1 + rate) ^ 10",
        "((((7))))",
        "--5",
        "1 / (x - 3)",
        "4 + ",
        "3 $ 4",
        "(1 + 2",
        "y + 1",
        "1 2",
    ];
    for e in exprs.iter() {
        match eval(e, &vars) {
            Ok(v) => println!("{:<24} = {}", e, (v * 1e6).round() / 1e6),
            Err(err) => println!("{:<24} ! {}", e, err),
        }
    }
    println!("{:?}", tokenize("a1+(2.5*b)").unwrap());
}
