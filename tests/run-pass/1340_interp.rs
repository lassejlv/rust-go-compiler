// A small tree-walking interpreter for a JavaScript-like toy language.
use std::cell::RefCell;
use std::collections::HashMap;
use std::fmt;
use std::rc::Rc;

#[derive(Debug, Clone, PartialEq)]
enum Tok {
    Num(f64),
    Str(String),
    Ident(String),
    Kw(&'static str),
    Op(&'static str),
    LParen,
    RParen,
    LBrace,
    RBrace,
    LBracket,
    RBracket,
    Comma,
    Semi,
    Eof,
}

const KEYWORDS: &[&str] = &["let", "fn", "if", "else", "while", "return", "true", "false", "nil", "print"];
const OPS: &[&str] = &["==", "!=", "<=", ">=", "&&", "||", "+", "-", "*", "/", "%", "<", ">", "=", "!"];

struct Lexer<'a> {
    src: &'a [u8],
    pos: usize,
    line: usize,
}

#[derive(Debug)]
enum LangError {
    Lex { line: usize, msg: String },
    Parse(String),
    Runtime(String),
}

impl fmt::Display for LangError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            LangError::Lex { line, msg } => write!(f, "lex error at line {}: {}", line, msg),
            LangError::Parse(m) => write!(f, "parse error: {}", m),
            LangError::Runtime(m) => write!(f, "runtime error: {}", m),
        }
    }
}

type Res<T> = Result<T, LangError>;

impl<'a> Lexer<'a> {
    fn new(s: &'a str) -> Self {
        Lexer { src: s.as_bytes(), pos: 0, line: 1 }
    }

    fn peek(&self) -> Option<u8> {
        self.src.get(self.pos).copied()
    }

    fn tokens(mut self) -> Res<Vec<Tok>> {
        let mut out = Vec::new();
        loop {
            while let Some(c) = self.peek() {
                if c == b'\n' {
                    self.line += 1;
                    self.pos += 1;
                } else if c.is_ascii_whitespace() {
                    self.pos += 1;
                } else if c == b'#' {
                    while let Some(c) = self.peek() {
                        if c == b'\n' {
                            break;
                        }
                        self.pos += 1;
                    }
                } else {
                    break;
                }
            }
            let c = match self.peek() {
                None => {
                    out.push(Tok::Eof);
                    return Ok(out);
                }
                Some(c) => c,
            };
            if c.is_ascii_digit() {
                let start = self.pos;
                while matches!(self.peek(), Some(b'0'..=b'9') | Some(b'.')) {
                    self.pos += 1;
                }
                let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                let n: f64 = s.parse().map_err(|_| LangError::Lex { line: self.line, msg: format!("bad number {}", s) })?;
                out.push(Tok::Num(n));
                continue;
            }
            if c.is_ascii_alphabetic() || c == b'_' {
                let start = self.pos;
                while let Some(c) = self.peek() {
                    if c.is_ascii_alphanumeric() || c == b'_' {
                        self.pos += 1;
                    } else {
                        break;
                    }
                }
                let s = std::str::from_utf8(&self.src[start..self.pos]).unwrap();
                if let Some(k) = KEYWORDS.iter().find(|k| **k == s) {
                    out.push(Tok::Kw(k));
                } else {
                    out.push(Tok::Ident(s.to_string()));
                }
                continue;
            }
            if c == b'"' {
                self.pos += 1;
                let mut s = String::new();
                loop {
                    match self.peek() {
                        None => return Err(LangError::Lex { line: self.line, msg: "unterminated string".into() }),
                        Some(b'"') => {
                            self.pos += 1;
                            break;
                        }
                        Some(b'\\') => {
                            self.pos += 1;
                            let e = self.peek().unwrap_or(b'?');
                            s.push(match e {
                                b'n' => '\n',
                                b't' => '\t',
                                other => other as char,
                            });
                            self.pos += 1;
                        }
                        Some(ch) => {
                            s.push(ch as char);
                            self.pos += 1;
                        }
                    }
                }
                out.push(Tok::Str(s));
                continue;
            }
            let simple = match c {
                b'(' => Some(Tok::LParen),
                b')' => Some(Tok::RParen),
                b'{' => Some(Tok::LBrace),
                b'}' => Some(Tok::RBrace),
                b'[' => Some(Tok::LBracket),
                b']' => Some(Tok::RBracket),
                b',' => Some(Tok::Comma),
                b';' => Some(Tok::Semi),
                _ => None,
            };
            if let Some(t) = simple {
                self.pos += 1;
                out.push(t);
                continue;
            }
            let rest = &self.src[self.pos..];
            let op = OPS.iter().find(|op| rest.starts_with(op.as_bytes()));
            match op {
                Some(op) => {
                    self.pos += op.len();
                    out.push(Tok::Op(op));
                }
                None => {
                    return Err(LangError::Lex { line: self.line, msg: format!("unexpected char {:?}", c as char) });
                }
            }
        }
    }
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
    Unary(&'static str, Box<Expr>),
    Binary(Box<Expr>, &'static str, Box<Expr>),
    Assign(String, Box<Expr>),
    Call(Box<Expr>, Vec<Expr>),
    Lambda(Rc<FnDef>),
}

#[derive(Debug)]
struct FnDef {
    name: Option<String>,
    params: Vec<String>,
    body: Vec<Stmt>,
}

#[derive(Debug, Clone)]
enum Stmt {
    Let(String, Expr),
    Expr(Expr),
    Print(Vec<Expr>),
    If(Expr, Vec<Stmt>, Option<Vec<Stmt>>),
    While(Expr, Vec<Stmt>),
    Return(Option<Expr>),
    Fn(Rc<FnDef>),
    Block(Vec<Stmt>),
}

struct Parser {
    toks: Vec<Tok>,
    pos: usize,
}

impl Parser {
    fn peek(&self) -> &Tok {
        &self.toks[self.pos]
    }
    fn next(&mut self) -> Tok {
        let t = self.toks[self.pos].clone();
        if self.pos + 1 < self.toks.len() {
            self.pos += 1;
        }
        t
    }
    fn expect(&mut self, t: Tok) -> Res<()> {
        let got = self.next();
        if got == t {
            Ok(())
        } else {
            Err(LangError::Parse(format!("expected {:?}, got {:?}", t, got)))
        }
    }
    fn eat_op(&mut self, op: &str) -> bool {
        if let Tok::Op(o) = self.peek() {
            if *o == op {
                self.pos += 1;
                return true;
            }
        }
        false
    }

    fn program(&mut self) -> Res<Vec<Stmt>> {
        let mut v = vec![];
        while *self.peek() != Tok::Eof {
            v.push(self.stmt()?);
        }
        Ok(v)
    }

    fn block(&mut self) -> Res<Vec<Stmt>> {
        self.expect(Tok::LBrace)?;
        let mut v = vec![];
        while *self.peek() != Tok::RBrace {
            if *self.peek() == Tok::Eof {
                return Err(LangError::Parse("unexpected eof in block".to_string()));
            }
            v.push(self.stmt()?);
        }
        self.expect(Tok::RBrace)?;
        Ok(v)
    }

    fn fn_def(&mut self, name: Option<String>) -> Res<Rc<FnDef>> {
        self.expect(Tok::LParen)?;
        let mut params = vec![];
        while *self.peek() != Tok::RParen {
            match self.next() {
                Tok::Ident(n) => params.push(n),
                t => return Err(LangError::Parse(format!("bad param {:?}", t))),
            }
            if *self.peek() == Tok::Comma {
                self.next();
            }
        }
        self.expect(Tok::RParen)?;
        let body = self.block()?;
        Ok(Rc::new(FnDef { name, params, body }))
    }

    fn stmt(&mut self) -> Res<Stmt> {
        match self.peek().clone() {
            Tok::Kw("let") => {
                self.next();
                let name = match self.next() {
                    Tok::Ident(n) => n,
                    t => return Err(LangError::Parse(format!("expected name, got {:?}", t))),
                };
                if !self.eat_op("=") {
                    return Err(LangError::Parse("expected =".into()));
                }
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Let(name, e))
            }
            Tok::Kw("fn") => {
                self.next();
                let name = match self.next() {
                    Tok::Ident(n) => n,
                    t => return Err(LangError::Parse(format!("expected fn name, got {:?}", t))),
                };
                Ok(Stmt::Fn(self.fn_def(Some(name))?))
            }
            Tok::Kw("print") => {
                self.next();
                self.expect(Tok::LParen)?;
                let args = self.args(Tok::RParen)?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Print(args))
            }
            Tok::Kw("if") => {
                self.next();
                let c = self.expr()?;
                let t = self.block()?;
                let e = if *self.peek() == Tok::Kw("else") {
                    self.next();
                    if *self.peek() == Tok::Kw("if") {
                        Some(vec![self.stmt()?])
                    } else {
                        Some(self.block()?)
                    }
                } else {
                    None
                };
                Ok(Stmt::If(c, t, e))
            }
            Tok::Kw("while") => {
                self.next();
                let c = self.expr()?;
                let b = self.block()?;
                Ok(Stmt::While(c, b))
            }
            Tok::Kw("return") => {
                self.next();
                if *self.peek() == Tok::Semi {
                    self.next();
                    return Ok(Stmt::Return(None));
                }
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Return(Some(e)))
            }
            Tok::LBrace => Ok(Stmt::Block(self.block()?)),
            _ => {
                let e = self.expr()?;
                self.expect(Tok::Semi)?;
                Ok(Stmt::Expr(e))
            }
        }
    }

    fn args(&mut self, close: Tok) -> Res<Vec<Expr>> {
        let mut v = vec![];
        while *self.peek() != close {
            v.push(self.expr()?);
            if *self.peek() == Tok::Comma {
                self.next();
            } else {
                break;
            }
        }
        self.expect(close)?;
        Ok(v)
    }

    fn expr(&mut self) -> Res<Expr> {
        let lhs = self.binary(0)?;
        if let Tok::Op("=") = self.peek() {
            self.next();
            let rhs = self.expr()?;
            return match lhs {
                Expr::Var(n) => Ok(Expr::Assign(n, Box::new(rhs))),
                other => Err(LangError::Parse(format!("cannot assign to {:?}", other))),
            };
        }
        Ok(lhs)
    }

    fn prec(op: &str) -> Option<u8> {
        Some(match op {
            "||" => 1,
            "&&" => 2,
            "==" | "!=" => 3,
            "<" | ">" | "<=" | ">=" => 4,
            "+" | "-" => 5,
            "*" | "/" | "%" => 6,
            _ => return None,
        })
    }

    fn binary(&mut self, min: u8) -> Res<Expr> {
        let mut lhs = self.unary()?;
        loop {
            let op = match self.peek() {
                Tok::Op(o) => *o,
                _ => break,
            };
            let p = match Self::prec(op) {
                Some(p) if p > min => p,
                _ => break,
            };
            self.next();
            let rhs = self.binary(p)?;
            lhs = Expr::Binary(Box::new(lhs), op, Box::new(rhs));
        }
        Ok(lhs)
    }

    fn unary(&mut self) -> Res<Expr> {
        if self.eat_op("-") {
            return Ok(Expr::Unary("-", Box::new(self.unary()?)));
        }
        if self.eat_op("!") {
            return Ok(Expr::Unary("!", Box::new(self.unary()?)));
        }
        self.postfix()
    }

    fn postfix(&mut self) -> Res<Expr> {
        let mut e = self.primary()?;
        loop {
            match self.peek() {
                Tok::LParen => {
                    self.next();
                    let a = self.args(Tok::RParen)?;
                    e = Expr::Call(Box::new(e), a);
                }
                Tok::LBracket => {
                    self.next();
                    let i = self.expr()?;
                    self.expect(Tok::RBracket)?;
                    e = Expr::Index(Box::new(e), Box::new(i));
                }
                _ => return Ok(e),
            }
        }
    }

    fn primary(&mut self) -> Res<Expr> {
        match self.next() {
            Tok::Num(n) => Ok(Expr::Num(n)),
            Tok::Str(s) => Ok(Expr::Str(s)),
            Tok::Kw("true") => Ok(Expr::Bool(true)),
            Tok::Kw("false") => Ok(Expr::Bool(false)),
            Tok::Kw("nil") => Ok(Expr::Nil),
            Tok::Kw("fn") => Ok(Expr::Lambda(self.fn_def(None)?)),
            Tok::Ident(n) => Ok(Expr::Var(n)),
            Tok::LParen => {
                let e = self.expr()?;
                self.expect(Tok::RParen)?;
                Ok(e)
            }
            Tok::LBracket => Ok(Expr::List(self.args(Tok::RBracket)?)),
            t => Err(LangError::Parse(format!("unexpected token {:?}", t))),
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
    Closure(Rc<FnDef>, Rc<RefCell<Env>>),
    Native(&'static str, fn(&[Value]) -> Res<Value>),
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Value::Num(n) => {
                if n.fract() == 0.0 && n.abs() < 1e15 {
                    write!(f, "{}", *n as i64)
                } else {
                    write!(f, "{:.4}", n)
                }
            }
            Value::Str(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => f.write_str("nil"),
            Value::List(l) => {
                write!(f, "[")?;
                for (i, v) in l.borrow().iter().enumerate() {
                    if i > 0 {
                        write!(f, ", ")?;
                    }
                    write!(f, "{}", v)?;
                }
                write!(f, "]")
            }
            Value::Closure(d, _) => write!(f, "<fn {}>", d.name.as_deref().unwrap_or("anon")),
            Value::Native(n, _) => write!(f, "<native {}>", n),
        }
    }
}

impl Value {
    fn truthy(&self) -> bool {
        !matches!(self, Value::Nil | Value::Bool(false))
    }
    fn type_name(&self) -> &'static str {
        match self {
            Value::Num(_) => "number",
            Value::Str(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::List(_) => "list",
            Value::Closure(..) | Value::Native(..) => "function",
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, o: &Value) -> bool {
        match (self, o) {
            (Value::Num(a), Value::Num(b)) => a == b,
            (Value::Str(a), Value::Str(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            (Value::List(a), Value::List(b)) => Rc::ptr_eq(a, b) || *a.borrow() == *b.borrow(),
            (Value::Closure(a, _), Value::Closure(b, _)) => Rc::ptr_eq(a, b),
            _ => false,
        }
    }
}

#[derive(Default)]
struct Env {
    vars: HashMap<String, Value>,
    parent: Option<Rc<RefCell<Env>>>,
}

impl Env {
    fn child(parent: &Rc<RefCell<Env>>) -> Rc<RefCell<Env>> {
        Rc::new(RefCell::new(Env { vars: HashMap::new(), parent: Some(Rc::clone(parent)) }))
    }
    fn get(&self, n: &str) -> Option<Value> {
        match self.vars.get(n) {
            Some(v) => Some(v.clone()),
            None => self.parent.as_ref().and_then(|p| p.borrow().get(n)),
        }
    }
    fn set(&mut self, n: &str, v: Value) -> bool {
        if let Some(slot) = self.vars.get_mut(n) {
            *slot = v;
            true
        } else if let Some(p) = &self.parent {
            p.borrow_mut().set(n, v)
        } else {
            false
        }
    }
}

enum Flow {
    Normal,
    Ret(Value),
}

struct Interp {
    out: Vec<String>,
    calls: usize,
    depth: usize,
}

fn native_len(a: &[Value]) -> Res<Value> {
    match a.first() {
        Some(Value::List(l)) => Ok(Value::Num(l.borrow().len() as f64)),
        Some(Value::Str(s)) => Ok(Value::Num(s.chars().count() as f64)),
        Some(v) => Err(LangError::Runtime(format!("len of {}", v.type_name()))),
        None => Err(LangError::Runtime("len needs 1 arg".into())),
    }
}

fn native_push(a: &[Value]) -> Res<Value> {
    if let [Value::List(l), v] = a {
        l.borrow_mut().push(v.clone());
        Ok(Value::Nil)
    } else {
        Err(LangError::Runtime("push(list, v)".into()))
    }
}

fn native_str(a: &[Value]) -> Res<Value> {
    let s: String = a.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("");
    Ok(Value::Str(s.into()))
}

fn native_floor(a: &[Value]) -> Res<Value> {
    match a {
        [Value::Num(n)] => Ok(Value::Num(n.floor())),
        _ => Err(LangError::Runtime("floor(num)".into())),
    }
}

impl Interp {
    fn globals() -> Rc<RefCell<Env>> {
        let env = Rc::new(RefCell::new(Env::default()));
        let natives: [(&'static str, fn(&[Value]) -> Res<Value>); 4] =
            [("len", native_len), ("push", native_push), ("str", native_str), ("floor", native_floor)];
        for (n, f) in natives.iter() {
            env.borrow_mut().vars.insert(n.to_string(), Value::Native(n, *f));
        }
        env
    }

    fn exec_block(&mut self, stmts: &[Stmt], env: &Rc<RefCell<Env>>) -> Res<Flow> {
        for s in stmts {
            if let Flow::Ret(v) = self.exec(s, env)? {
                return Ok(Flow::Ret(v));
            }
        }
        Ok(Flow::Normal)
    }

    fn exec(&mut self, s: &Stmt, env: &Rc<RefCell<Env>>) -> Res<Flow> {
        match s {
            Stmt::Let(n, e) => {
                let v = self.eval(e, env)?;
                env.borrow_mut().vars.insert(n.clone(), v);
            }
            Stmt::Expr(e) => {
                self.eval(e, env)?;
            }
            Stmt::Print(args) => {
                let mut parts = Vec::with_capacity(args.len());
                for a in args {
                    parts.push(self.eval(a, env)?.to_string());
                }
                self.out.push(parts.join(" "));
            }
            Stmt::If(c, t, e) => {
                if self.eval(c, env)?.truthy() {
                    return self.exec_block(t, &Env::child(env));
                } else if let Some(e) = e {
                    return self.exec_block(e, &Env::child(env));
                }
            }
            Stmt::While(c, b) => {
                let mut guard = 0u32;
                while self.eval(c, env)?.truthy() {
                    guard += 1;
                    if guard > 1_000_000 {
                        return Err(LangError::Runtime("loop limit".into()));
                    }
                    if let Flow::Ret(v) = self.exec_block(b, &Env::child(env))? {
                        return Ok(Flow::Ret(v));
                    }
                }
            }
            Stmt::Return(e) => {
                let v = match e {
                    Some(e) => self.eval(e, env)?,
                    None => Value::Nil,
                };
                return Ok(Flow::Ret(v));
            }
            Stmt::Fn(def) => {
                let name = def.name.clone().unwrap();
                let clo = Value::Closure(Rc::clone(def), Rc::clone(env));
                env.borrow_mut().vars.insert(name, clo);
            }
            Stmt::Block(b) => return self.exec_block(b, &Env::child(env)),
        }
        Ok(Flow::Normal)
    }

    fn call(&mut self, f: Value, args: Vec<Value>) -> Res<Value> {
        self.calls += 1;
        match f {
            Value::Native(_, nf) => nf(&args),
            Value::Closure(def, cenv) => {
                if def.params.len() != args.len() {
                    return Err(LangError::Runtime(format!(
                        "{} expects {} args, got {}",
                        def.name.as_deref().unwrap_or("lambda"),
                        def.params.len(),
                        args.len()
                    )));
                }
                if self.depth > 200 {
                    return Err(LangError::Runtime("stack overflow".into()));
                }
                let local = Env::child(&cenv);
                for (p, a) in def.params.iter().zip(args) {
                    local.borrow_mut().vars.insert(p.clone(), a);
                }
                self.depth += 1;
                let r = self.exec_block(&def.body, &local);
                self.depth -= 1;
                match r? {
                    Flow::Ret(v) => Ok(v),
                    Flow::Normal => Ok(Value::Nil),
                }
            }
            other => Err(LangError::Runtime(format!("cannot call {}", other.type_name()))),
        }
    }

    fn eval(&mut self, e: &Expr, env: &Rc<RefCell<Env>>) -> Res<Value> {
        Ok(match e {
            Expr::Num(n) => Value::Num(*n),
            Expr::Str(s) => Value::Str(Rc::from(s.as_str())),
            Expr::Bool(b) => Value::Bool(*b),
            Expr::Nil => Value::Nil,
            Expr::Var(n) => env.borrow().get(n).ok_or_else(|| LangError::Runtime(format!("undefined variable {}", n)))?,
            Expr::List(items) => {
                let vs = items.iter().map(|i| self.eval(i, env)).collect::<Res<Vec<_>>>()?;
                Value::List(Rc::new(RefCell::new(vs)))
            }
            Expr::Index(l, i) => {
                let l = self.eval(l, env)?;
                let i = self.eval(i, env)?;
                match (&l, &i) {
                    (Value::List(v), Value::Num(n)) => {
                        let idx = *n as usize;
                        v.borrow().get(idx).cloned().ok_or_else(|| LangError::Runtime(format!("index {} out of range", idx)))?
                    }
                    (Value::Str(s), Value::Num(n)) => match s.chars().nth(*n as usize) {
                        Some(c) => Value::Str(c.to_string().into()),
                        None => Value::Nil,
                    },
                    _ => return Err(LangError::Runtime(format!("cannot index {} with {}", l.type_name(), i.type_name()))),
                }
            }
            Expr::Unary(op, x) => {
                let v = self.eval(x, env)?;
                match (*op, v) {
                    ("-", Value::Num(n)) => Value::Num(-n),
                    ("!", v) => Value::Bool(!v.truthy()),
                    (op, v) => return Err(LangError::Runtime(format!("bad unary {} on {}", op, v.type_name()))),
                }
            }
            Expr::Binary(a, "&&", b) => {
                let l = self.eval(a, env)?;
                if !l.truthy() {
                    l
                } else {
                    self.eval(b, env)?
                }
            }
            Expr::Binary(a, "||", b) => {
                let l = self.eval(a, env)?;
                if l.truthy() {
                    l
                } else {
                    self.eval(b, env)?
                }
            }
            Expr::Binary(a, op, b) => {
                let l = self.eval(a, env)?;
                let r = self.eval(b, env)?;
                binop(op, l, r)?
            }
            Expr::Assign(n, x) => {
                let v = self.eval(x, env)?;
                if !env.borrow_mut().set(n, v.clone()) {
                    return Err(LangError::Runtime(format!("assign to undeclared {}", n)));
                }
                v
            }
            Expr::Call(f, args) => {
                let fv = self.eval(f, env)?;
                let mut av = Vec::new();
                for a in args {
                    av.push(self.eval(a, env)?);
                }
                self.call(fv, av)?
            }
            Expr::Lambda(def) => Value::Closure(def.clone(), env.clone()),
        })
    }
}

fn binop(op: &str, l: Value, r: Value) -> Res<Value> {
    use Value::*;
    Ok(match (op, &l, &r) {
        ("+", Num(a), Num(b)) => Num(a + b),
        ("+", Str(a), b) => Str(format!("{}{}", a, b).into()),
        ("+", List(a), List(b)) => {
            let mut v = a.borrow().clone();
            v.extend(b.borrow().iter().cloned());
            List(Rc::new(RefCell::new(v)))
        }
        ("-", Num(a), Num(b)) => Num(a - b),
        ("*", Num(a), Num(b)) => Num(a * b),
        ("*", Str(a), Num(b)) => Str(a.repeat(*b as usize).into()),
        ("/", Num(_), Num(b)) if *b == 0.0 => return Err(LangError::Runtime("division by zero".into())),
        ("/", Num(a), Num(b)) => Num(a / b),
        ("%", Num(a), Num(b)) => Num(a % b),
        ("<", Num(a), Num(b)) => Bool(a < b),
        (">", Num(a), Num(b)) => Bool(a > b),
        ("<=", Num(a), Num(b)) => Bool(a <= b),
        (">=", Num(a), Num(b)) => Bool(a >= b),
        ("<", Str(a), Str(b)) => Bool(a < b),
        ("==", _, _) => Bool(l == r),
        ("!=", _, _) => Bool(l != r),
        _ => return Err(LangError::Runtime(format!("bad operands for {}: {} and {}", op, l.type_name(), r.type_name()))),
    })
}

fn run(src: &str) -> (Vec<String>, Result<usize, String>) {
    let toks = match Lexer::new(src).tokens() {
        Ok(t) => t,
        Err(e) => return (vec![], Err(e.to_string())),
    };
    let mut p = Parser { toks, pos: 0 };
    let prog = match p.program() {
        Ok(p) => p,
        Err(e) => return (vec![], Err(e.to_string())),
    };
    let mut it = Interp { out: vec![], calls: 0, depth: 0 };
    let g = Interp::globals();
    let r = it.exec_block(&prog, &g).map(|_| it.calls).map_err(|e| e.to_string());
    (it.out, r)
}

const PROGRAMS: &[&str] = &[
    r#"
fn fib(n) { if n < 2 { return n; } return fib(n - 1) + fib(n - 2); }
let i = 0;
let acc = [];
while i < 15 { push(acc, fib(i)); i = i + 1; }
print("fibs:", acc);
"#,
    r#"
fn make_counter(start) {
  let n = start;
  return fn() { n = n + 1; return n; };
}
let a = make_counter(10);
let b = make_counter(100);
a(); a(); b();
print("counters", a(), b(), a());
"#,
    r#"
fn map(xs, f) { let out = []; let i = 0; while i < len(xs) { push(out, f(xs[i])); i = i + 1; } return out; }
fn filter(xs, f) { let out = []; let i = 0; while i < len(xs) { if f(xs[i]) { push(out, xs[i]); } i = i + 1; } return out; }
fn reduce(xs, f, init) { let acc = init; let i = 0; while i < len(xs) { acc = f(acc, xs[i]); i = i + 1; } return acc; }
let xs = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10];
let sq = map(xs, fn(x) { return x * x; });
let ev = filter(sq, fn(x) { return x % 2 == 0; });
print(sq, ev, reduce(ev, fn(a, b) { return a + b; }, 0));
print("avg", reduce(xs, fn(a, b) { return a + b; }, 0) / len(xs));
"#,
    r#"
# string building and recursion
fn rev(s) { if len(s) == 0 { return ""; } let i = len(s) - 1; let out = ""; while i >= 0 { out = out + s[i]; i = i - 1; } return out; }
print(rev("hello world"), "ab" * 3, str("x=", 1.5, ",", true));
fn collatz(n) { let steps = 0; while n != 1 { if n % 2 == 0 { n = n / 2; } else { n = 3 * n + 1; } steps = steps + 1; } return steps; }
let best = 0; let arg = 0; let k = 1;
while k < 300 { let c = collatz(k); if c > best { best = c; arg = k; } k = k + 1; }
print("collatz", arg, best);
"#,
    r#"
fn compose(f, g) { return fn(x) { return f(g(x)); }; }
let inc = fn(x) { return x + 1; };
let dbl = fn(x) { return x * 2; };
let h = compose(inc, compose(dbl, inc));
print(h(5), compose(dbl, dbl)(3), inc);
let fs = [];
let j = 0;
while j < 4 { let m = j; push(fs, fn() { return m * 10; }); j = j + 1; }
print(fs[0](), fs[1](), fs[3]());
"#,
    r#"
print("before");
let x = 1 / 0;
print("unreachable");
"#,
    r#"
fn f(a, b) { return a; }
print(f(1));
"#,
    r#"let s = "abc; print(s);"#,
    r#"let = 5;"#,
    r#"
fn deep(n) { return deep(n + 1); }
deep(0);
"#,
    r#"
let primes = [];
let n = 2;
while len(primes) < 25 {
  let isp = true; let i = 0;
  while i < len(primes) && primes[i] * primes[i] <= n {
    if n % primes[i] == 0 { isp = false; }
    i = i + 1;
  }
  if isp { push(primes, n); }
  n = n + 1;
}
print(primes, floor(7.9), 7 / 2, -3.25 * 2, !nil, [1,2] + [3], [1, [2, 3]] == [1, [2, 3]]);
if 1 > 2 { print("no"); } else if 2 > 1 { print("elseif"); } else { print("else"); }
"#,
];

fn main() {
    let mut total_calls = 0;
    for (i, p) in PROGRAMS.iter().enumerate() {
        let (out, res) = run(p);
        println!("== program {} ==", i);
        for line in &out {
            println!("  {}", line);
        }
        match res {
            Ok(calls) => {
                total_calls += calls;
                println!("  ok ({} calls)", calls);
            }
            Err(e) => println!("  error: {}", e),
        }
    }
    println!("total calls: {}", total_calls);
}
