// A small stack VM with a bytecode compiler for an expression/statement
// language with functions, locals, loops and conditionals. Includes a
// disassembler, a constant folder and execution statistics.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Push(i64),
    Load(u8),
    Store(u8),
    LoadG(u8),
    StoreG(u8),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Lt,
    Eq,
    Not,
    Jmp(u16),
    Jz(u16),
    Call(u16, u8),
    Ret,
    Print,
    Pop,
    Halt,
}

impl fmt::Display for Op {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Op::Push(v) => write!(f, "push {}", v),
            Op::Load(i) => write!(f, "load ${}", i),
            Op::Store(i) => write!(f, "store ${}", i),
            Op::LoadG(i) => write!(f, "loadg @{}", i),
            Op::StoreG(i) => write!(f, "storeg @{}", i),
            Op::Jmp(t) => write!(f, "jmp {:04}", t),
            Op::Jz(t) => write!(f, "jz {:04}", t),
            Op::Call(t, n) => write!(f, "call {:04}/{}", t, n),
            other => write!(f, "{}", format!("{:?}", other).to_lowercase()),
        }
    }
}

#[derive(Debug, Clone)]
enum E {
    Num(i64),
    Var(String),
    Bin(char, Box<E>, Box<E>),
    Neg(Box<E>),
    Call(String, Vec<E>),
}

#[derive(Debug, Clone)]
enum S {
    Let(String, E),
    Set(String, E),
    Print(E),
    If(E, Vec<S>, Vec<S>),
    While(E, Vec<S>),
    Ret(E),
    Expr(E),
}

struct Func {
    name: String,
    params: Vec<String>,
    body: Vec<S>,
}

// ---------- parser (s-expression-ish surface syntax) ----------
fn tokenize(s: &str) -> Vec<String> {
    let mut toks = Vec::new();
    let mut cur = String::new();
    for c in s.chars() {
        if c == '(' || c == ')' {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
            toks.push(c.to_string());
        } else if c.is_whitespace() {
            if !cur.is_empty() {
                toks.push(std::mem::take(&mut cur));
            }
        } else {
            cur.push(c);
        }
    }
    if !cur.is_empty() {
        toks.push(cur);
    }
    toks
}

#[derive(Debug, Clone)]
enum Sx {
    Atom(String),
    List(Vec<Sx>),
}

fn read(toks: &[String], pos: &mut usize) -> Result<Sx, String> {
    let t = toks.get(*pos).ok_or("unexpected eof")?;
    *pos += 1;
    match t.as_str() {
        "(" => {
            let mut v = Vec::new();
            while toks.get(*pos).map(|s| s.as_str()) != Some(")") {
                if *pos >= toks.len() {
                    return Err("missing )".into());
                }
                v.push(read(toks, pos)?);
            }
            *pos += 1;
            Ok(Sx::List(v))
        }
        ")" => Err("unexpected )".into()),
        a => Ok(Sx::Atom(a.to_string())),
    }
}

fn to_expr(s: &Sx) -> Result<E, String> {
    match s {
        Sx::Atom(a) => Ok(match a.parse::<i64>() {
            Ok(n) => E::Num(n),
            Err(_) => E::Var(a.clone()),
        }),
        Sx::List(v) => {
            let head = match v.first() {
                Some(Sx::Atom(h)) => h.as_str(),
                _ => return Err("bad call head".into()),
            };
            let args: Result<Vec<E>, String> = v[1..].iter().map(to_expr).collect();
            let mut args = args?;
            match head {
                "+" | "-" | "*" | "/" | "%" | "<" | "=" | ">" if args.len() >= 2 => {
                    let op = head.chars().next().unwrap();
                    let first = args.remove(0);
                    Ok(args.into_iter().fold(first, |acc, x| E::Bin(op, Box::new(acc), Box::new(x))))
                }
                "-" if args.len() == 1 => Ok(E::Neg(Box::new(args.remove(0)))),
                _ => Ok(E::Call(head.to_string(), args)),
            }
        }
    }
}

fn to_stmts(v: &[Sx]) -> Result<Vec<S>, String> {
    v.iter().map(to_stmt).collect()
}

fn to_stmt(s: &Sx) -> Result<S, String> {
    if let Sx::List(v) = s {
        if let Some(Sx::Atom(h)) = v.first() {
            let name = |i: usize| match v.get(i) {
                Some(Sx::Atom(n)) => Ok(n.clone()),
                _ => Err(format!("{}: expected name", h)),
            };
            let body = |i: usize| match v.get(i) {
                Some(Sx::List(b)) => to_stmts(b),
                None => Ok(vec![]),
                _ => Err(format!("{}: expected block", h)),
            };
            return match h.as_str() {
                "let" => Ok(S::Let(name(1)?, to_expr(&v[2])?)),
                "set" => Ok(S::Set(name(1)?, to_expr(&v[2])?)),
                "print" => Ok(S::Print(to_expr(&v[1])?)),
                "if" => Ok(S::If(to_expr(&v[1])?, body(2)?, body(3)?)),
                "while" => Ok(S::While(to_expr(&v[1])?, body(2)?)),
                "return" => Ok(S::Ret(to_expr(&v[1])?)),
                _ => Ok(S::Expr(to_expr(s)?)),
            };
        }
    }
    Ok(S::Expr(to_expr(s)?))
}

fn parse_program(src: &str) -> Result<(Vec<Func>, Vec<S>), String> {
    let toks = tokenize(src);
    let mut pos = 0;
    let mut funcs = Vec::new();
    let mut main = Vec::new();
    while pos < toks.len() {
        let sx = read(&toks, &mut pos)?;
        if let Sx::List(v) = &sx {
            if let Some(Sx::Atom(h)) = v.first() {
                if h == "def" {
                    let name = match &v[1] { Sx::Atom(n) => n.clone(), _ => return Err("def name".into()) };
                    let params = match &v[2] {
                        Sx::List(ps) => ps.iter().map(|p| match p { Sx::Atom(a) => Ok(a.clone()), _ => Err("param".to_string()) }).collect::<Result<_, _>>()?,
                        _ => return Err("def params".into()),
                    };
                    let body = match &v[3] { Sx::List(b) => to_stmts(b)?, _ => return Err("def body".into()) };
                    funcs.push(Func { name, params, body });
                    continue;
                }
            }
        }
        main.push(to_stmt(&sx)?);
    }
    Ok((funcs, main))
}

// ---------- constant folding ----------
fn fold(e: E) -> E {
    match e {
        E::Bin(op, a, b) => {
            let (a, b) = (fold(*a), fold(*b));
            match (&a, &b) {
                (E::Num(x), E::Num(y)) => match op {
                    '+' => E::Num(x.wrapping_add(*y)),
                    '-' => E::Num(x.wrapping_sub(*y)),
                    '*' => E::Num(x.wrapping_mul(*y)),
                    '/' if *y != 0 => E::Num(x / y),
                    '%' if *y != 0 => E::Num(x % y),
                    '<' => E::Num((x < y) as i64),
                    '>' => E::Num((x > y) as i64),
                    '=' => E::Num((x == y) as i64),
                    _ => E::Bin(op, Box::new(a), Box::new(b)),
                },
                (_, E::Num(0)) if op == '+' || op == '-' => a,
                (_, E::Num(1)) if op == '*' => a,
                _ => E::Bin(op, Box::new(a), Box::new(b)),
            }
        }
        E::Neg(x) => match fold(*x) {
            E::Num(n) => E::Num(-n),
            other => E::Neg(Box::new(other)),
        },
        E::Call(n, args) => E::Call(n, args.into_iter().map(fold).collect()),
        other => other,
    }
}

// ---------- compiler ----------
struct Compiler {
    code: Vec<Op>,
    globals: HashMap<String, u8>,
    locals: Vec<String>,
    funcs: HashMap<String, (u16, u8)>,
    fixups: Vec<(usize, String, usize)>,
    in_func: bool,
}

impl Compiler {
    fn var_slot(&self, n: &str) -> Option<Op> {
        if self.in_func {
            if let Some(i) = self.locals.iter().rposition(|l| l == n) {
                return Some(Op::Load(i as u8));
            }
        }
        self.globals.get(n).map(|&g| Op::LoadG(g))
    }

    fn expr(&mut self, e: &E) -> Result<(), String> {
        match e {
            E::Num(n) => self.code.push(Op::Push(*n)),
            E::Var(v) => {
                let op = self.var_slot(v).ok_or_else(|| format!("unknown variable {}", v))?;
                self.code.push(op);
            }
            E::Neg(x) => {
                self.expr(x)?;
                self.code.push(Op::Neg);
            }
            E::Bin(op, a, b) => {
                if *op == '>' {
                    self.expr(b)?;
                    self.expr(a)?;
                    self.code.push(Op::Lt);
                    return Ok(());
                }
                self.expr(a)?;
                self.expr(b)?;
                self.code.push(match op {
                    '+' => Op::Add,
                    '-' => Op::Sub,
                    '*' => Op::Mul,
                    '/' => Op::Div,
                    '%' => Op::Mod,
                    '<' => Op::Lt,
                    '=' => Op::Eq,
                    _ => return Err(format!("bad op {}", op)),
                });
            }
            E::Call(n, args) => {
                if n == "not" && args.len() == 1 {
                    self.expr(&args[0])?;
                    self.code.push(Op::Not);
                    return Ok(());
                }
                for a in args {
                    self.expr(a)?;
                }
                let at = self.code.len();
                self.code.push(Op::Call(0, args.len() as u8));
                self.fixups.push((at, n.clone(), args.len()));
            }
        }
        Ok(())
    }

    fn stmts(&mut self, ss: &[S]) -> Result<(), String> {
        for s in ss {
            self.stmt(s)?;
        }
        Ok(())
    }

    fn stmt(&mut self, s: &S) -> Result<(), String> {
        match s {
            S::Let(n, e) => {
                self.expr(e)?;
                if self.in_func {
                    self.locals.push(n.clone());
                    self.code.push(Op::Store((self.locals.len() - 1) as u8));
                } else {
                    let next = self.globals.len() as u8;
                    let g = *self.globals.entry(n.clone()).or_insert(next);
                    self.code.push(Op::StoreG(g));
                }
            }
            S::Set(n, e) => {
                self.expr(e)?;
                match self.var_slot(n) {
                    Some(Op::Load(i)) => self.code.push(Op::Store(i)),
                    Some(Op::LoadG(g)) => self.code.push(Op::StoreG(g)),
                    _ => return Err(format!("set of unknown variable {}", n)),
                }
            }
            S::Print(e) => {
                self.expr(e)?;
                self.code.push(Op::Print);
            }
            S::Expr(e) => {
                self.expr(e)?;
                self.code.push(Op::Pop);
            }
            S::Ret(e) => {
                self.expr(e)?;
                self.code.push(Op::Ret);
            }
            S::If(c, t, f) => {
                self.expr(c)?;
                let jz = self.code.len();
                self.code.push(Op::Jz(0));
                self.stmts(t)?;
                if f.is_empty() {
                    let here = self.code.len() as u16;
                    self.code[jz] = Op::Jz(here);
                } else {
                    let jmp = self.code.len();
                    self.code.push(Op::Jmp(0));
                    self.code[jz] = Op::Jz(self.code.len() as u16);
                    self.stmts(f)?;
                    self.code[jmp] = Op::Jmp(self.code.len() as u16);
                }
            }
            S::While(c, body) => {
                let top = self.code.len() as u16;
                self.expr(c)?;
                let jz = self.code.len();
                self.code.push(Op::Jz(0));
                self.stmts(body)?;
                self.code.push(Op::Jmp(top));
                self.code[jz] = Op::Jz(self.code.len() as u16);
            }
        }
        Ok(())
    }
}

fn fold_stmts(ss: Vec<S>) -> Vec<S> {
    ss.into_iter()
        .map(|s| match s {
            S::Let(n, e) => S::Let(n, fold(e)),
            S::Set(n, e) => S::Set(n, fold(e)),
            S::Print(e) => S::Print(fold(e)),
            S::Ret(e) => S::Ret(fold(e)),
            S::Expr(e) => S::Expr(fold(e)),
            S::If(c, t, f) => S::If(fold(c), fold_stmts(t), fold_stmts(f)),
            S::While(c, b) => S::While(fold(c), fold_stmts(b)),
        })
        .collect()
}

fn compile(src: &str, optimize: bool) -> Result<Vec<Op>, String> {
    let (funcs, mut main) = parse_program(src)?;
    if optimize {
        main = fold_stmts(main);
    }
    let mut c = Compiler { code: vec![], globals: HashMap::new(), locals: vec![], funcs: HashMap::new(), fixups: vec![], in_func: false };
    c.stmts(&main)?;
    c.code.push(Op::Halt);
    for f in funcs {
        let body = if optimize { fold_stmts(f.body) } else { f.body };
        c.funcs.insert(f.name.clone(), (c.code.len() as u16, f.params.len() as u8));
        c.in_func = true;
        c.locals = f.params.clone();
        c.stmts(&body)?;
        c.code.push(Op::Push(0));
        c.code.push(Op::Ret);
    }
    for (at, name, argc) in std::mem::take(&mut c.fixups) {
        let &(addr, arity) = c.funcs.get(&name).ok_or_else(|| format!("unknown function {}", name))?;
        if arity as usize != argc {
            return Err(format!("{} expects {} args, got {}", name, arity, argc));
        }
        c.code[at] = Op::Call(addr, arity);
    }
    Ok(c.code)
}

#[derive(Debug)]
enum VmError { StackUnderflow(usize), DivZero(usize), StepLimit, CallDepth }

struct Frame { ret: usize, base: usize }

fn run(code: &[Op], out: &mut Vec<i64>, limit: u64) -> Result<(u64, usize), VmError> {
    let mut stack: Vec<i64> = Vec::with_capacity(256);
    let mut globals = [0i64; 32];
    let mut frames: Vec<Frame> = Vec::new();
    let mut pc = 0usize;
    let mut steps = 0u64;
    let mut max_stack = 0;
    macro_rules! pop {
        () => {
            stack.pop().ok_or(VmError::StackUnderflow(pc))?
        };
    }
    loop {
        steps += 1;
        if steps > limit {
            return Err(VmError::StepLimit);
        }
        max_stack = max_stack.max(stack.len());
        let op = code[pc];
        pc += 1;
        match op {
            Op::Push(v) => stack.push(v),
            Op::Load(i) => {
                let base = frames.last().map_or(0, |f| f.base);
                stack.push(stack[base + i as usize]);
            }
            Op::Store(i) => {
                let v = pop!();
                let base = frames.last().map_or(0, |f| f.base);
                let idx = base + i as usize;
                if idx == stack.len() {
                    stack.push(v);
                } else {
                    stack[idx] = v;
                }
            }
            Op::LoadG(g) => stack.push(globals[g as usize]),
            Op::StoreG(g) => globals[g as usize] = pop!(),
            Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Lt | Op::Eq => {
                let b = pop!();
                let a = pop!();
                stack.push(match op {
                    Op::Add => a.wrapping_add(b),
                    Op::Sub => a.wrapping_sub(b),
                    Op::Mul => a.wrapping_mul(b),
                    Op::Div => a.checked_div(b).ok_or(VmError::DivZero(pc - 1))?,
                    Op::Mod => a.checked_rem(b).ok_or(VmError::DivZero(pc - 1))?,
                    Op::Lt => (a < b) as i64,
                    Op::Eq => (a == b) as i64,
                    _ => unreachable!(),
                });
            }
            Op::Neg => {
                let a = pop!();
                stack.push(a.wrapping_neg());
            }
            Op::Not => {
                let a = pop!();
                stack.push((a == 0) as i64);
            }
            Op::Jmp(t) => pc = t as usize,
            Op::Jz(t) => {
                if pop!() == 0 {
                    pc = t as usize;
                }
            }
            Op::Call(t, n) => {
                if frames.len() > 200 {
                    return Err(VmError::CallDepth);
                }
                frames.push(Frame { ret: pc, base: stack.len() - n as usize });
                pc = t as usize;
            }
            Op::Ret => {
                let v = pop!();
                let f = frames.pop().ok_or(VmError::StackUnderflow(pc))?;
                stack.truncate(f.base);
                stack.push(v);
                pc = f.ret;
            }
            Op::Print => out.push(pop!()),
            Op::Pop => {
                pop!();
            }
            Op::Halt => return Ok((steps, max_stack)),
        }
    }
}

fn demo(name: &str, src: &str) {
    println!("== {} ==", name);
    for opt in [false, true] {
        match compile(src, opt) {
            Err(e) => {
                println!("  compile error: {}", e);
                return;
            }
            Ok(code) => {
                let mut out = Vec::new();
                let r = run(&code, &mut out, 2_000_000);
                println!("  opt={:<5} ops={:<4} result={:?} output={:?}", opt, code.len(), r, out);
            }
        }
    }
}

fn main() {
    let src = "(let x (+ 1 2 3)) (print (* x (- 10 4))) (print (- x)) (print (> x 5)) (print (not (= x 6)))";
    let code = compile(src, false).unwrap();
    for (i, op) in code.iter().enumerate() {
        println!("{:04}  {}", i, op);
    }
    demo("arith", src);
    demo("folding", "(print (+ (* 2 3) (* 4 5) (- 0 7))) (let y 9) (print (+ y 0)) (print (* (+ y 1) 1))");
    demo(
        "loops",
        "(let i 0) (let s 0) (while (< i 100) ((set s (+ s (* i i))) (set i (+ i 1)))) (print s)
         (let n 27) (let steps 0) (while (> n 1) ((if (= (% n 2) 0) ((set n (/ n 2))) ((set n (+ (* 3 n) 1)))) (set steps (+ steps 1)))) (print steps)",
    );
    demo(
        "functions",
        "(def fact (n) ((if (< n 2) ((return 1))) (return (* n (fact (- n 1))))))
         (def fib (n) ((if (< n 2) ((return n))) (return (+ (fib (- n 1)) (fib (- n 2))))))
         (def gcd (a b) ((while (> b 0) ((let t (% a b)) (set a b) (set b t))) (return a)))
         (def noret (a) ((print a)))
         (print (fact 20)) (print (fib 18)) (print (gcd 1071 462)) (print (noret 5))
         (let k 0) (while (< k 5) ((print (fact k)) (set k (+ k 1))))",
    );
    demo(
        "primes",
        "(def isprime (n) ((if (< n 2) ((return 0))) (let d 2) (while (< (* d d) (+ n 1)) ((if (= (% n d) 0) ((return 0))) (set d (+ d 1)))) (return 1)))
         (let c 0) (let i 0) (let last 0) (while (< i 2000) ((if (isprime i) ((set c (+ c 1)) (set last i))) (set i (+ i 1)))) (print c) (print last)",
    );
    demo("overflow wraps", "(let a 9223372036854775807) (print (+ a 1)) (print (* a 2)) (print (- 0 a 2))");
    demo("div zero", "(let z 0) (print (/ 10 z))");
    demo("infinite", "(while 1 ((let q 1)))");
    demo("deep recursion", "(def f (n) ((return (f (+ n 1))))) (print (f 0))");
    demo("unknown var", "(print (+ q 1))");
    demo("unknown fn", "(print (g 1))");
    demo("arity", "(def h (a b) ((return a))) (print (h 1))");
    demo("parse error", "(print (+ 1 2)");
}
