use std::error::Error;
use std::fmt;

#[derive(Debug)]
enum LexError { UnexpectedChar(char, usize), UnterminatedString(usize) }
#[derive(Debug)]
enum ParseError { Lex(LexError), UnexpectedToken { found: String, expected: &'static str }, Eof }
#[derive(Debug)]
enum EvalError { Parse(ParseError), DivZero, Unknown(String), Depth(usize) }

impl fmt::Display for LexError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LexError::UnexpectedChar(c, p) => write!(f, "unexpected char {:?} at {}", c, p),
            LexError::UnterminatedString(p) => write!(f, "unterminated string starting at {}", p),
        }
    }
}
impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ParseError::Lex(_) => write!(f, "lexing failed"),
            ParseError::UnexpectedToken { found, expected } => write!(f, "expected {}, found {}", expected, found),
            ParseError::Eof => write!(f, "unexpected end of input"),
        }
    }
}
impl fmt::Display for EvalError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            EvalError::Parse(_) => write!(f, "could not parse"),
            EvalError::DivZero => write!(f, "division by zero"),
            EvalError::Unknown(n) => write!(f, "unknown variable `{}`", n),
            EvalError::Depth(d) => write!(f, "nesting too deep ({})", d),
        }
    }
}
impl Error for LexError {}
impl Error for ParseError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self { ParseError::Lex(e) => Some(e), _ => None } } }
impl Error for EvalError { fn source(&self) -> Option<&(dyn Error + 'static)> { match self { EvalError::Parse(e) => Some(e), _ => None } } }
impl From<LexError> for ParseError { fn from(e: LexError) -> Self { ParseError::Lex(e) } }
impl From<ParseError> for EvalError { fn from(e: ParseError) -> Self { EvalError::Parse(e) } }

#[derive(Debug, Clone, PartialEq)]
enum Tok { Num(i64), Id(String), Str(String), Op(char), LP, RP }

fn lex(s: &str) -> Result<Vec<Tok>, LexError> {
    let cs: Vec<char> = s.chars().collect();
    let mut i = 0;
    let mut out = vec![];
    while i < cs.len() {
        let c = cs[i];
        match c {
            ' ' => { i += 1; }
            '0'..='9' => { let st = i; while i < cs.len() && cs[i].is_ascii_digit() { i += 1; } out.push(Tok::Num(cs[st..i].iter().collect::<String>().parse().unwrap())); }
            'a'..='z' => { let st = i; while i < cs.len() && cs[i].is_ascii_alphanumeric() { i += 1; } out.push(Tok::Id(cs[st..i].iter().collect())); }
            '"' => { let st = i; i += 1; while i < cs.len() && cs[i] != '"' { i += 1; } if i >= cs.len() { return Err(LexError::UnterminatedString(st)); } out.push(Tok::Str(cs[st + 1..i].iter().collect())); i += 1; }
            '+' | '-' | '*' | '/' => { out.push(Tok::Op(c)); i += 1; }
            '(' => { out.push(Tok::LP); i += 1; }
            ')' => { out.push(Tok::RP); i += 1; }
            _ => return Err(LexError::UnexpectedChar(c, i)),
        }
    }
    Ok(out)
}

#[derive(Debug)]
enum Ast { Num(i64), Var(String), Bin(Box<Ast>, char, Box<Ast>), Len(String) }

struct P { t: Vec<Tok>, i: usize }
impl P {
    fn peek(&self) -> Option<&Tok> { self.t.get(self.i) }
    fn next(&mut self) -> Result<Tok, ParseError> { let t = self.t.get(self.i).cloned().ok_or(ParseError::Eof)?; self.i += 1; Ok(t) }
    fn expr(&mut self) -> Result<Ast, ParseError> {
        let mut l = self.term()?;
        while let Some(Tok::Op(c @ ('+' | '-'))) = self.peek().cloned() { self.i += 1; let r = self.term()?; l = Ast::Bin(Box::new(l), c, Box::new(r)); }
        Ok(l)
    }
    fn term(&mut self) -> Result<Ast, ParseError> {
        let mut l = self.atom()?;
        while let Some(&Tok::Op(c)) = self.peek() { if c != '*' && c != '/' { break; } self.i += 1; let r = self.atom()?; l = Ast::Bin(Box::new(l), c, Box::new(r)); }
        Ok(l)
    }
    fn atom(&mut self) -> Result<Ast, ParseError> {
        match self.next()? {
            Tok::Num(n) => Ok(Ast::Num(n)),
            Tok::Id(s) => Ok(Ast::Var(s)),
            Tok::Str(s) => Ok(Ast::Len(s)),
            Tok::LP => { let e = self.expr()?; match self.next()? { Tok::RP => Ok(e), t => Err(ParseError::UnexpectedToken { found: format!("{:?}", t), expected: "')'" }) } }
            t => Err(ParseError::UnexpectedToken { found: format!("{:?}", t), expected: "atom" }),
        }
    }
}

fn eval(a: &Ast, vars: &[(&str, i64)], depth: usize) -> Result<i64, EvalError> {
    if depth > 6 { return Err(EvalError::Depth(depth)); }
    Ok(match a {
        Ast::Num(n) => *n,
        Ast::Len(s) => s.chars().count() as i64,
        Ast::Var(v) => vars.iter().find(|(n, _)| n == v).map(|(_, x)| *x).ok_or_else(|| EvalError::Unknown(v.clone()))?,
        Ast::Bin(l, op, r) => {
            let (x, y) = (eval(l, vars, depth + 1)?, eval(r, vars, depth + 1)?);
            match op { '+' => x + y, '-' => x - y, '*' => x * y, '/' => x.checked_div(y).ok_or(EvalError::DivZero)?, _ => unreachable!() }
        }
    })
}

fn run(src: &str) -> Result<i64, EvalError> {
    let toks = lex(src).map_err(ParseError::from)?;
    let mut p = P { t: toks, i: 0 };
    let ast = p.expr()?;
    if let Some(t) = p.peek() { return Err(ParseError::UnexpectedToken { found: format!("{:?}", t), expected: "end" }.into()); }
    eval(&ast, &[("x", 6), ("y", -2)], 0)
}

fn run_boxed(src: &str) -> Result<String, Box<dyn Error>> {
    let v = run(src)?;
    let half: u8 = u8::try_from(v / 2)?;
    Ok(format!("half={}", half))
}

fn chain(e: &dyn Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    while let Some(c) = cur { s.push_str(" <- "); s.push_str(&c.to_string()); cur = c.source(); }
    s
}

fn main() {
    let cases = ["1 + 2 * 3", "(1 + 2) * 3", "x * y - \"héllo\"", "10 / (x - 6)", "z + 1", "1 + $", "\"abc", "(1 + 2", "1 2", "((((((((1))))))))", "1+1+1+1+1+1+1+1", "", "x*x*x*x*x*x*x*x"];
    for c in cases {
        match run(c) {
            Ok(v) => println!("{:<22} = {}", c, v),
            Err(e) => println!("{:<22} ! {} | chain: {} | {:?}", c, e, chain(&e), e),
        }
    }
    for c in ["x * 20", "x * 100", "y", "q"] {
        match run_boxed(c) { Ok(s) => println!("{} -> {}", c, s), Err(e) => println!("{} -> boxed err: {}", c, e) }
    }
    let r: Result<Vec<i64>, EvalError> = ["1", "2+x", "3"].iter().map(|s| run(s)).collect();
    println!("{:?}", r);
    let r2: Result<Vec<i64>, String> = ["1", "2/0", "3"].iter().map(|s| run(s).map_err(|e| e.to_string())).collect();
    println!("{:?}", r2);
    let e: Box<dyn Error> = Box::new(EvalError::Parse(ParseError::Lex(LexError::UnterminatedString(4))));
    println!("{}", chain(e.as_ref()));
    match e.downcast::<EvalError>() { Ok(ev) => println!("downcast ok: {:?}", ev), Err(_) => println!("downcast failed") }
}
