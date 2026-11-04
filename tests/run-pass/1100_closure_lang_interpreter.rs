// A tokenizer + recursive-descent parser + tree-walking interpreter for a small
// expression language with let-bindings, closures, recursion and lists.
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;
use std::cell::RefCell;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    True,
    False,
    Nil,
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Assign,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    Or,
    Not,
    Arrow,
    Eof,
}

struct Lexer<'a> {
    chars: std::iter::Peekable<std::str::CharIndices<'a>>,
    src: &'a str,
    line: usize,
}

impl<'a> Lexer<'a> {
    fn new(src: &'a str) -> Self {
        Lexer { chars: src.char_indices().peekable(), src, line: 1 }
    }

    fn tokens(mut self) -> Result<Vec<(Tok, usize)>, String> {
        let mut out = Vec::new();
        loop {
            let t = self.next_tok()?;
            let done = t == Tok::Eof;
            out.push((t, self.line));
            if done {
                break;
            }
        }
        Ok(out)
    }

    fn next_tok(&mut self) -> Result<Tok, String> {
        // skip whitespace and comments
        loop {
            match self.chars.peek() {
                Some(&(_, c)) if c.is_whitespace() => {
                    if c == '\n' {
                        self.line += 1;
                    }
                    self.chars.next();
                }
                Some(&(_, '#')) => {
                    while let Some(&(_, c)) = self.chars.peek() {
                        if c == '\n' {
                            break;
                        }
                        self.chars.next();
                    }
                }
                _ => break,
            }
        }
        let (start, c) = match self.chars.next() {
            None => return Ok(Tok::Eof),
            Some(p) => p,
        };
        let two = |s: &mut Self, next: char, yes: Tok, no: Tok| -> Tok {
            if let Some(&(_, c2)) = s.chars.peek() {
                if c2 == next {
                    s.chars.next();
                    return yes;
                }
            }
            no
        };
        let t = match c {
            '(' => Tok::LParen,
            ')' => Tok::RParen,
            '{' => Tok::LBrace,
            '}' => Tok::RBrace,
            '[' => Tok::LBracket,
            ']' => Tok::RBracket,
            ',' => Tok::Comma,
            ';' => Tok::Semi,
            '+' => Tok::Plus,
            '-' => two(self, '>', Tok::Arrow, Tok::Minus),
            '*' => Tok::Star,
            '/' => Tok::Slash,
            '%' => Tok::Percent,
            '=' => two(self, '=', Tok::Eq, Tok::Assign),
            '!' => two(self, '=', Tok::Ne, Tok::Not),
            '<' => two(self, '=', Tok::Le, Tok::Lt),
            '>' => two(self, '=', Tok::Ge, Tok::Gt),
            '&' => {
                if two(self, '&', Tok::And, Tok::Eof) == Tok::And {
                    Tok::And
                } else {
                    return Err(format!("line {}: lone '&'", self.line));
                }
            }
            '|' => {
                if two(self, '|', Tok::Or, Tok::Eof) == Tok::Or {
                    Tok::Or
                } else {
                    return Err(format!("line {}: lone '|'", self.line));
                }
            }
            '"' => {
                let mut s = String::new();
                loop {
                    match self.chars.next() {
                        None => return Err(format!("line {}: unterminated string", self.line)),
                        Some((_, '"')) => break,
                        Some((_, '\\')) => match self.chars.next() {
                            Some((_, 'n')) => s.push('\n'),
                            Some((_, 't')) => s.push('\t'),
                            Some((_, c)) => s.push(c),
                            None => return Err("bad escape".to_string()),
                        },
                        Some((_, c)) => s.push(c),
                    }
                }
                Tok::Str(s)
            }
            c if c.is_ascii_digit() => {
                let mut end = start + 1;
                while let Some(&(i, c)) = self.chars.peek() {
                    if c.is_ascii_digit() || c == '.' {
                        end = i + 1;
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                let text = &self.src[start..end];
                Tok::Num(text.parse().map_err(|e| format!("bad number {:?}: {}", text, e))?)
            }
            c if c.is_alphabetic() || c == '_' => {
                let mut end = start + c.len_utf8();
                while let Some(&(i, c)) = self.chars.peek() {
                    if c.is_alphanumeric() || c == '_' {
                        end = i + c.len_utf8();
                        self.chars.next();
                    } else {
                        break;
                    }
                }
                match &self.src[start..end] {
                    "let" => Tok::Let,
                    "fn" => Tok::Fn,
                    "if" => Tok::If,
                    "else" => Tok::Else,
                    "while" => Tok::While,
                    "return" => Tok::Return,
                    "true" => Tok::True,
                    "false" => Tok::False,
                    "nil" => Tok::Nil,
                    id => Tok::Ident(id.to_string()),
                }
            }
            other => return Err(format!("line {}: unexpected char {:?}", self.line, other)),
        };
        Ok(t)
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum BinOp {
    Add, Sub, Mul, Div, Mod, Eq, Ne, Lt, Le, Gt, Ge, And, Or,
}

#[derive(Debug, Clone)]
enum Expr {
    Num(f64),
    Str(String),
    Bool(bool),
    Nil,
    Var(String),
    List(Vec<Expr>),
    Index(Box<Expr>, Box<Expr>),
    Unary(char, Box<Expr>),
    Bin(BinOp, Box<Expr>, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Lambda(Rc<Vec<String>>, Rc<Vec<Stmt>>),
    If(Box<Expr>, Vec<Stmt>, Vec<Stmt>),
}

#[derive(Debug, Clone)]
enum Stmt {
    Let(String, Expr),
    Assign(String, Expr),
    Expr(Expr),
    While(Expr, Vec<Stmt>),
    Return(Expr),
    FnDecl(String, Rc<Vec<String>>, Rc<Vec<Stmt>>),
}

struct Parser {
    toks: Vec<(Tok, usize)>,
    pos: usize,
}

type PResult<T> = Result<T, String>;

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos].0
    }
    fn peek2(&self) -> &Tok {
        &self.toks[(self.pos + 1).min(self.toks.len() - 1)].0
    }
    fn bump(&mut self) -> Tok {
        let t = self.toks[self.pos].0.clone();
        if self.pos < self.toks.len() - 1 {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: Tok) -> PResult<()> {
        if *self.peek() == t {
            self.bump();
            Ok(())
        } else {
            Err(format!("line {}: expected {:?}, found {:?}", self.toks[self.pos].1, t, self.peek()))
        }
    }
    fn ident(&mut self) -> PResult<String> {
        match self.bump() {
            Tok::Ident(s) => Ok(s),
            t => Err(format!("expected identifier, got {:?}", t)),
        }
    }

    fn program(&mut self) -> PResult<Vec<Stmt>> {
        let mut v = Vec::new();
        while *self.peek() != Tok::Eof {
            v.push(self.stmt()?);
        }
        Ok(v)
    }

    fn block(&mut self) -> PResult<Vec<Stmt>> {
        self.expect(Tok::LBrace)?;
        let mut v = Vec::new();
        while *self.peek() != Tok::RBrace {
            v.push(self.stmt()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(v)
    }

    fn params(&mut self) -> PResult<Vec<String>> {
        self.expect(Tok::LParen)?;
        let mut ps = Vec::new();
        while *self.peek() != Tok::RParen {
            ps.push(self.ident()?);
            if *self.peek() == Tok::Comma {
                self.bump();
            }
        }
        self.expect(Tok::RParen)?;
        Ok(ps)
    }

    fn stmt(&mut self) -> PResult<Stmt> {
        match self.peek().clone() {
            Tok::Let => {
                self.bump();
                let name = self.ident()?;
                self.expect(Tok::Assign)?;
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Let(name, e))
            }
            Tok::Fn if matches!(self.peek2(), Tok::Ident(_)) => {
                self.bump();
                let name = self.ident()?;
                let ps = self.params()?;
                let body = self.block()?;
                Ok(Stmt::FnDecl(name, Rc::new(ps), Rc::new(body)))
            }
            Tok::While => {
                self.bump();
                let c = self.expr()?;
                let b = self.block()?;
                Ok(Stmt::While(c, b))
            }
            Tok::Return => {
                self.bump();
                let e = if *self.peek() == Tok::Semi { Expr::Nil } else { self.expr()? };
                self.expect(Tok::Semi)?;
                Ok(Stmt::Return(e))
            }
            Tok::Ident(name) if *self.peek2() == Tok::Assign => {
                self.bump();
                self.bump();
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Assign(name, e))
            }
            _ => {
                let e = self.expr()?;
                if *self.peek() == Tok::Semi {
                    self.bump();
                }
                Ok(Stmt::Expr(e))
            }
        }
    }

    fn expr(&mut self) -> PResult<Expr> {
        self.binary(0)
    }

    fn prec(t: &Tok) -> Option<(u8, BinOp)> {
        Some(match t {
            Tok::Or => (1, BinOp::Or),
            Tok::And => (2, BinOp::And),
            Tok::Eq => (3, BinOp::Eq),
            Tok::Ne => (3, BinOp::Ne),
            Tok::Lt => (4, BinOp::Lt),
            Tok::Le => (4, BinOp::Le),
            Tok::Gt => (4, BinOp::Gt),
            Tok::Ge => (4, BinOp::Ge),
            Tok::Plus => (5, BinOp::Add),
            Tok::Minus => (5, BinOp::Sub),
            Tok::Star => (6, BinOp::Mul),
            Tok::Slash => (6, BinOp::Div),
            Tok::Percent => (6, BinOp::Mod),
            _ => return None,
        })
    }

    fn binary(&mut self, min: u8) -> PResult<Expr> {
        let mut lhs = self.unary()?;
        while let Some((p, op)) = Self::prec(self.peek()) {
            if p <= min {
                break;
            }
            self.bump();
            let rhs = self.binary(p)?;
            lhs = Expr::Bin(op, Box::new(lhs), Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> PResult<Expr> {
        match self.peek() {
            Tok::Minus => {
                self.bump();
                Ok(Expr::Unary('-', Box::new(self.unary()?)))
            }
            Tok::Not => {
                self.bump();
                Ok(Expr::Unary('!', Box::new(self.unary()?)))
            }
            _ => self.postfix(),
        }
    }

    fn postfix(&mut self) -> PResult<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::LParen => {
                    self.bump();
                    let mut args = Vec::new();
                    while *self.peek() != Tok::RParen {
                        args.push(self.expr()?);
                        if *self.peek() == Tok::Comma {
                            self.bump();
                        }
                    }
                    self.expect(Tok::RParen)?;
                    e = Expr::Call(Box::new(e), args);
                }
                Tok::LBracket => {
                    self.bump();
                    let i = self.expr()?;
                    self.expect(Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(i));
                }
                _ => return Ok(e),
            }
        }
    }

    fn primary(&mut self) -> PResult<Expr> {
        match self.bump() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::True => Ok(Expr::Bool(true)),
            Tok::False => Ok(Expr::Bool(false)),
            Tok::Nil => Ok(Expr::Nil),
            Tok::Ident(s) => Ok(Expr::Var(s)),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => {
                let mut items = Vec::new();
                while *self.peek() != Tok::RBracket {
                    items.push(self.expr()?);
                    if *self.peek() == Tok::Comma {
                        self.bump();
                    }
                }
                self.expect(Tok::RBracket)?;
                Ok(Expr::List(items))
            }
            Tok::Fn => {
                let ps = self.params()?;
                if *self.peek() == Tok::Arrow {
                    self.bump();
                    let e = self.expr()?;
                    Ok(Expr::Lambda(Rc::new(ps), Rc::new(vec![Stmt::Return(e)])))
                } else {
                    let b = self.block()?;
                    Ok(Expr::Lambda(Rc::new(ps), Rc::new(b)))
                }
            }
            Tok::If => {
                let c = self.expr()?;
                let t = self.block()?;
                let f = if *self.peek() == Tok::Else {
                    self.bump();
                    if *self.peek() == Tok::If {
                        vec![Stmt::Expr(self.primary()?)]
                    } else {
                        self.block()?
                    }
                } else {
                    vec![]
                };
                Ok(Expr::If(Box::new(c), t, f))
            }
            t => Err(format!("unexpected token {:?}", t)),
        }
    }
}

type EnvRef = Rc<RefCell<Env>>;

struct Env {
    vars: HashMap<String, Value>,
    parent: Option<EnvRef>,
}

impl Env {
    fn new(parent: Option<EnvRef>) -> EnvRef {
        Rc::new(RefCell::new(Env { vars: HashMap::new(), parent }))
    }
    fn get(env: &EnvRef, name: &str) -> Option<Value> {
        let e = env.borrow();
        if let Some(v) = e.vars.get(name) {
            return Some(v.clone());
        }
        match &e.parent {
            Some(p) => Env::get(p, name),
            None => None,
        }
    }
    fn set(env: &EnvRef, name: &str, v: Value) -> bool {
        let mut e = env.borrow_mut();
        if e.vars.contains_key(name) {
            e.vars.insert(name.to_string(), v);
            return true;
        }
        match &e.parent {
            Some(p) => Env::set(p, name, v),
            None => false,
        }
    }
}

#[derive(Clone)]
enum Value {
    Num(f64),
    Str(Rc<str>),
    Bool(bool),
    Nil,
    List(Rc<RefCell<Vec<Value>>>),
    Closure(Rc<Vec<String>>, Rc<Vec<Stmt>>, EnvRef),
    Builtin(&'static str, fn(&[Value]) -> Result<Value, String>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::List(items) => {
                write!(f, "[")?;
                for (i, it) in items.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    match it {
                        Value::Str(s) => write!(f, "{:?}", &**s)?,
                        other => write!(f, "{}", other)?,
                    }
                }
                write!(f, "]")
            }
            Value::Closure(ps, _, _) => write!(f, "<fn/{}>", ps.len()),
            Value::Builtin(n, _) => write!(f, "<builtin {}>", n),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
    fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "num",
            Value::Str(_) => "str",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::List(_) => "list",
            Value::Closure(..) | Value::Builtin(..) => "fn",
        }
    }
}

enum Flow {
    Normal(Value),
    Return(Value),
}

struct Interp {
    calls: usize,
    output: Vec<String>,
}

fn b_len(a: &[Value]) -> Result<Value, String> {
    match a.first() {
        Some(Value::List(l)) => Ok(Value::Num(l.borrow().len() as f64)),
        Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
        _ => Err("len: bad argument".into()),
    }
}
fn b_push(a: &[Value]) -> Result<Value, String> {
    match a {
        [Value::List(l), v] => {
            l.borrow_mut().push(v.clone());
            Ok(Value::Nil)
        }
        _ => Err("push: bad arguments".into()),
    }
}
fn b_str(a: &[Value]) -> Result<Value, String> {
    let s: String = a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("");
    Ok(Value::Str(s.into()))
}
fn b_type(a: &[Value]) -> Result<Value, String> {
    Ok(Value::Str(a.first().map(|v| v.type_name()).unwrap_or("nil").into()))
}
fn b_floor(a: &[Value]) -> Result<Value, String> {
    match a.first() {
        Some(Value::Num(n)) => Ok(Value::Num(n.floor())),
        _ => Err("floor: expected num".into()),
    }
}

impl Interp {
    fn global() -> EnvRef {
        let g = Env::new(None);
        {
            let mut e = g.borrow_mut();
            let bs: [(&'static str, fn(&[Value]) -> Result<Value, String>); 5] =
                [("len", b_len), ("push", b_push), ("str", b_str), ("type", b_type), ("floor", b_floor)];
            for (n, f) in bs {
                e.vars.insert(n.to_string(), Value::Builtin(n, f));
            }
        }
        g
    }

    fn exec_block(&mut self, stmts: &[Stmt], env: &EnvRef) -> Result<Flow, String> {
        let mut last = Value::Nil;
        for s in stmts {
            match self.exec(s, env)? {
                Flow::Return(v) => return Ok(Flow::Return(v)),
                Flow::Normal(v) => last = v,
            }
        }
        Ok(Flow::Normal(last))
    }

    fn exec(&mut self, s: &Stmt, env: &EnvRef) -> Result<Flow, String> {
        match s {
            Stmt::Let(n, e) => {
                let v = self.eval(e, env)?;
                env.borrow_mut().vars.insert(n.clone(), v);
                Ok(Flow::Normal(Value::Nil))
            }
            Stmt::Assign(n, e) => {
                let v = self.eval(e, env)?;
                if !Env::set(env, n, v) {
                    return Err(format!("assignment to undefined variable '{}'", n));
                }
                Ok(Flow::Normal(Value::Nil))
            }
            Stmt::Expr(e) => Ok(Flow::Normal(self.eval(e, env)?)),
            Stmt::While(c, body) => {
                while self.eval(c, env)?.truthy() {
                    let inner = Env::new(Some(env.clone()));
                    if let Flow::Return(v) = self.exec_block(body, &inner)? {
                        return Ok(Flow::Return(v));
                    }
                }
                Ok(Flow::Normal(Value::Nil))
            }
            Stmt::Return(e) => Ok(Flow::Return(self.eval(e, env)?)),
            Stmt::FnDecl(n, ps, body) => {
                let c = Value::Closure(ps.clone(), body.clone(), env.clone());
                env.borrow_mut().vars.insert(n.clone(), c);
                Ok(Flow::Normal(Value::Nil))
            }
        }
    }

    fn call(&mut self, f: Value, args: Vec<Value>) -> Result<Value, String> {
        self.calls += 1;
        match f {
            Value::Builtin(name, func) => {
                if name == "print" {
                    unreachable!()
                }
                func(&args)
            }
            Value::Closure(ps, body, cenv) => {
                if ps.len() != args.len() {
                    return Err(format!("arity mismatch: expected {}, got {}", ps.len(), args.len()));
                }
                let env = Env::new(Some(cenv));
                for (p, a) in ps.iter().zip(args) {
                    env.borrow_mut().vars.insert(p.clone(), a);
                }
                match self.exec_block(&body, &env)? {
                    Flow::Return(v) | Flow::Normal(v) => Ok(v),
                }
            }
            other => Err(format!("cannot call a {}", other.type_name())),
        }
    }

    fn eval(&mut self, e: &Expr, env: &EnvRef) -> Result<Value, String> {
        Ok(match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Str(s.as_str().into()),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Nil => Value::Nil,
            Expr::Var(n) => {
                if n == "print" {
                    return Ok(Value::Builtin("print", |_| Ok(Value::Nil)));
                }
                Env::get(env, n).ok_or_else(|| format!("undefined variable '{}'", n))?
            }
            Expr::List(items) => {
                let mut v = Vec::with_capacity(items.len());
                for it in items {
                    v.push(self.eval(it, env)?);
                }
                Value::List(Rc::new(RefCell::new(v)))
            }
            Expr::Index(l, i) => {
                let lv = self.eval(l, env)?;
                let iv = self.eval(i, env)?;
                match (lv, iv) {
                    (Value::List(l), Value::Num(n)) => {
                        let l = l.borrow();
                        let idx = if n < 0.0 { l.len() as f64 + n } else { n } as usize;
                        l.get(idx).cloned().ok_or_else(|| format!("index {} out of range (len {})", n, l.len()))?
                    }
                    (Value::Str(s), Value::Num(n)) => match s.chars().nth(n as usize) {
                        Some(c) => Value::Str(c.to_string().into()),
                        None => Value::Nil,
                    },
                    (a, b) => return Err(format!("cannot index {} with {}", a.type_name(), b.type_name())),
                }
            }
            Expr::Unary(op, x) => {
                let v = self.eval(x, env)?;
                match (op, v) {
                    ('-', Value::Num(n)) => Value::Num(-n),
                    ('!', v) => Value::Bool(!v.truthy()),
                    (_, v) => return Err(format!("bad operand for unary {}: {}", op, v.type_name())),
                }
            }
            Expr::Bin(BinOp::And, a, b) => {
                let av = self.eval(a, env)?;
                if !av.truthy() { av } else { self.eval(b, env)? }
            }
            Expr::Bin(BinOp::Or, a, b) => {
                let av = self.eval(a, env)?;
                if av.truthy() { av } else { self.eval(b, env)? }
            }
            Expr::Bin(op, a, b) => {
                let av = self.eval(a, env)?;
                let bv = self.eval(b, env)?;
                binop(*op, av, bv)?
            }
            Expr::Call(fe, args) => {
                if let Expr::Var(n) = &**fe {
                    if n == "print" {
                        let mut parts = Vec::new();
                        for a in args {
                            parts.push(self.eval(a, env)?.to_string());
                        }
                        let line = parts.join(" ");
                        println!("{}", line);
                        self.output.push(line);
                        return Ok(Value::Nil);
                    }
                }
                let f = self.eval(fe, env)?;
                let mut av = Vec::new();
                for a in args {
                    av.push(self.eval(a, env)?);
                }
                self.call(f, av)?
            }
            Expr::Lambda(ps, body) => Value::Closure(ps.clone(), body.clone(), env.clone()),
            Expr::If(c, t, f) => {
                let branch = if self.eval(c, env)?.truthy() { t } else { f };
                let inner = Env::new(Some(env.clone()));
                match self.exec_block(branch, &inner)? {
                    Flow::Normal(v) => v,
                    Flow::Return(v) => return Err(format!("__return__{}", v)),
                }
            }
        })
    }
}

fn binop(op: BinOp, a: Value, b: Value) -> Result<Value, String> {
    use BinOp::*;
    Ok(match (op, &a, &b) {
        (Add, Value::Num(x), Value::Num(y)) => Value::Num(x + y),
        (Sub, Value::Num(x), Value::Num(y)) => Value::Num(x - y),
        (Mul, Value::Num(x), Value::Num(y)) => Value::Num(x * y),
        (Div, Value::Num(_), Value::Num(y)) if *y == 0.0 => return Err("division by zero".into()),
        (Div, Value::Num(x), Value::Num(y)) => Value::Num(x / y),
        (Mod, Value::Num(x), Value::Num(y)) => Value::Num(x % y),
        (Add, Value::Str(x), _) => Value::Str(format!("{}{}", x, b).into()),
        (Add, Value::List(x), Value::List(y)) => {
            let mut v = x.borrow().clone();
            v.extend(y.borrow().iter().cloned());
            Value::List(Rc::new(RefCell::new(v)))
        }
        (Mul, Value::Str(s), Value::Num(n)) => Value::Str(s.repeat(*n as usize).into()),
        (Lt, Value::Num(x), Value::Num(y)) => Value::Bool(x < y),
        (Le, Value::Num(x), Value::Num(y)) => Value::Bool(x <= y),
        (Gt, Value::Num(x), Value::Num(y)) => Value::Bool(x > y),
        (Ge, Value::Num(x), Value::Num(y)) => Value::Bool(x >= y),
        (Lt, Value::Str(x), Value::Str(y)) => Value::Bool(x < y),
        (Eq, _, _) => Value::Bool(eq(&a, &b)),
        (Ne, _, _) => Value::Bool(!eq(&a, &b)),
        _ => return Err(format!("type error: {:?} on {} and {}", op, a.type_name(), b.type_name())),
    })
}

fn eq(a: &Value, b: &Value) -> bool {
    match (a, b) {
        (Value::Num(x), Value::Num(y)) => x == y,
        (Value::Str(x), Value::Str(y)) => x == y,
        (Value::Bool(x), Value::Bool(y)) => x == y,
        (Value::Nil, Value::Nil) => true,
        (Value::List(x), Value::List(y)) => {
            let (x, y) = (x.borrow(), y.borrow());
            x.len() == y.len() && x.iter().zip(y.iter()).all(|(a, b)| eq(a, b))
        }
        _ => false,
    }
}

fn run(name: &str, src: &str) {
    println!("=== {} ===", name);
    let toks = match Lexer::new(src).tokens() {
        Ok(t) => t,
        Err(e) => {
            println!("lex error: {}", e);
            return;
        }
    };
    let mut p = Parser { toks, pos: 0 };
    let prog = match p.program() {
        Ok(p) => p,
        Err(e) => {
            println!("parse error: {}", e);
            return;
        }
    };
    let mut it = Interp { calls: 0, output: Vec::new() };
    let g = Interp::global();
    match it.exec_block(&prog, &g) {
        Ok(Flow::Normal(v)) | Ok(Flow::Return(v)) => {
            println!("-> {} (calls={}, lines={})", v, it.calls, it.output.len())
        }
        Err(e) => println!("runtime error: {} (after {} calls)", e, it.calls),
    }
}

fn main() {
    run("arith", "let x = 2 + 3 * 4 - 10 / 4; print(x); x * 2");
    run(
        "fib",
        r#"
        fn fib(n) { if n < 2 { n } else { fib(n - 1) + fib(n - 2) } }
        let i = 0;
        let out = [];
        while i < 15 { push(out, fib(i)); i = i + 1; }
        print(out);
        fib(20)
        "#,
    );
    run(
        "closures",
        r#"
        fn make_counter() {
            let n = 0;
            return fn() { n = n + 1; return n; };
        }
        let a = make_counter();
        let b = make_counter();
        a(); a(); b();
        print("a:", a(), "b:", b());
        let adder = fn(x) -> fn(y) -> x + y;
        let add5 = adder(5);
        print(add5(10), adder(1)(2));
        "done"
        "#,
    );
    run(
        "higher-order",
        r#"
        fn map(xs, f) { let out = []; let i = 0; while i < len(xs) { push(out, f(xs[i])); i = i + 1; } return out; }
        fn filter(xs, f) { let out = []; let i = 0; while i < len(xs) { if f(xs[i]) { push(out, xs[i]); } i = i + 1; } return out; }
        fn reduce(xs, f, acc) { let i = 0; while i < len(xs) { acc = f(acc, xs[i]); i = i + 1; } return acc; }
        fn range(a, b) { let out = []; while a < b { push(out, a); a = a + 1; } return out; }
        let xs = range(1, 21);
        let sq = map(xs, fn(x) -> x * x);
        let evens = filter(sq, fn(x) -> x % 2 == 0);
        print("squares of evens:", evens);
        print("sum:", reduce(evens, fn(a, b) -> a + b, 0));
        let words = ["pear", "fig", "banana", "kiwi"];
        print(map(words, fn(w) -> w + "!" ), map(words, len));
        reduce(map(words, len), fn(a, b) -> if a > b { a } else { b }, 0)
        "#,
    );
    run(
        "strings",
        r#"
        let s = "héllo";
        print(len(s), s[1], "ab" * 3, type(s), type(1), type([]), type(len));
        let acc = "";
        let i = 0;
        while i < 5 { acc = acc + str(i, ":"); i = i + 1; }
        print(acc, acc == "0:1:2:3:4:", [1, [2, "x"]] == [1, [2, "x"]]);
        print(floor(7 / 2), 7 / 2, -3 % 2, 1 / 3);
        "#,
    );
    run(
        "sieve",
        r#"
        fn primes(n) {
            let flags = [];
            let i = 0;
            while i <= n { push(flags, true); i = i + 1; }
            let out = [];
            let p = 2;
            while p <= n {
                if flags[p] {
                    push(out, p);
                    let m = p * p;
                    while m <= n { flags = set(flags, m); m = m + p; }
                }
                p = p + 1;
            }
            return out;
        }
        fn set(xs, i) {
            let out = []; let j = 0;
            while j < len(xs) { if j == i { push(out, false); } else { push(out, xs[j]); } j = j + 1; }
            return out;
        }
        let ps = primes(60);
        print(ps, len(ps), ps[-1]);
        "#,
    );
    run("err-undefined", "let a = 1; print(a); b + 1");
    run("err-type", "let a = [1,2]; a - 1");
    run("err-divzero", "fn f(x) { 10 / x } f(2); f(0)");
    run("err-arity", "fn f(x, y) { x } f(1)");
    run("err-parse", "let = 5;");
    run("err-lex", "let a = 1 & 2;");
    run("err-index", "[1,2,3][7]");
    run("comments", "# leading comment\nlet z = 4; # trailing\n z * z # final");
}
