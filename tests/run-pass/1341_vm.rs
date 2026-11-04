// Bytecode compiler + stack VM with a mark/sweep arena heap.
use std::collections::{BTreeMap, HashMap};
use std::fmt::{self, Write as _};

#[derive(Debug, Clone, Copy, PartialEq)]
enum Val {
    Int(i64),
    Bool(bool),
    Ref(HeapId),
    Unit,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct HeapId(u32);

#[derive(Debug)]
enum Obj {
    Pair(Val, Val),
    Array(Vec<Val>),
    Str(String),
}

struct Slot {
    obj: Option<Obj>,
    marked: bool,
}

#[derive(Default)]
struct Heap {
    slots: Vec<Slot>,
    free: Vec<u32>,
    allocs: usize,
    collections: usize,
    freed_total: usize,
}

impl Heap {
    fn alloc(&mut self, o: Obj) -> HeapId {
        self.allocs += 1;
        if let Some(i) = self.free.pop() {
            self.slots[i as usize] = Slot { obj: Some(o), marked: false };
            HeapId(i)
        } else {
            self.slots.push(Slot { obj: Some(o), marked: false });
            HeapId(self.slots.len() as u32 - 1)
        }
    }
    fn get(&self, id: HeapId) -> &Obj {
        self.slots[id.0 as usize].obj.as_ref().expect("dangling ref")
    }
    fn get_mut(&mut self, id: HeapId) -> &mut Obj {
        self.slots[id.0 as usize].obj.as_mut().expect("dangling ref")
    }
    fn live(&self) -> usize {
        self.slots.iter().filter(|s| s.obj.is_some()).count()
    }
    fn collect<'a, I: IntoIterator<Item = &'a Val>>(&mut self, roots: I) -> usize {
        self.collections += 1;
        let mut work: Vec<HeapId> = roots
            .into_iter()
            .filter_map(|v| if let Val::Ref(h) = v { Some(*h) } else { None })
            .collect();
        while let Some(h) = work.pop() {
            let slot = &mut self.slots[h.0 as usize];
            if slot.marked {
                continue;
            }
            slot.marked = true;
            let mut push = |v: &Val| {
                if let Val::Ref(c) = v {
                    work.push(*c);
                }
            };
            match slot.obj.as_ref().unwrap() {
                Obj::Pair(a, b) => {
                    push(a);
                    push(b);
                }
                Obj::Array(xs) => xs.iter().for_each(push),
                Obj::Str(_) => {}
            }
        }
        let mut freed = 0;
        for (i, s) in self.slots.iter_mut().enumerate() {
            if s.obj.is_some() && !s.marked {
                s.obj = None;
                self.free.push(i as u32);
                freed += 1;
            }
            s.marked = false;
        }
        self.freed_total += freed;
        freed
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Push(i64),
    PushBool(bool),
    PushUnit,
    Load(u16),
    Store(u16),
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Lt,
    Eq,
    Not,
    Jmp(u32),
    Jz(u32),
    Call(u16, u8),
    Ret,
    Pop,
    Cons,
    Car,
    Cdr,
    NewArr(u16),
    ArrGet,
    ArrSet,
    ArrLen,
    Str(u16),
    Concat,
    Print,
    Halt,
}

#[derive(Debug, Clone)]
enum Ast {
    Int(i64),
    Bool(bool),
    Var(String),
    Let(String, Box<Ast>, Box<Ast>),
    Set(String, Box<Ast>),
    Bin(char, Box<Ast>, Box<Ast>),
    If(Box<Ast>, Box<Ast>, Box<Ast>),
    While(Box<Ast>, Box<Ast>),
    Seq(Vec<Ast>),
    Call(String, Vec<Ast>),
    Cons(Box<Ast>, Box<Ast>),
    Car(Box<Ast>),
    Cdr(Box<Ast>),
    Arr(Vec<Ast>),
    Index(Box<Ast>, Box<Ast>),
    SetIndex(Box<Ast>, Box<Ast>, Box<Ast>),
    Len(Box<Ast>),
    Str(String),
    Print(Box<Ast>),
    Unit,
}

struct FnProto {
    name: String,
    params: Vec<String>,
    body: Ast,
}

struct Compiled {
    code: Vec<Op>,
    entry: HashMap<String, (u32, u16)>,
    strings: Vec<String>,
}

#[derive(Debug)]
enum CompileError {
    Unbound(String),
    UnknownFn(String),
    Arity { name: String, want: usize, got: usize },
}

impl fmt::Display for CompileError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            CompileError::Unbound(v) => write!(f, "unbound variable `{}`", v),
            CompileError::UnknownFn(v) => write!(f, "unknown function `{}`", v),
            CompileError::Arity { name, want, got } => write!(f, "`{}` takes {} args but {} given", name, want, got),
        }
    }
}

struct FnCompiler<'a> {
    code: &'a mut Vec<Op>,
    scopes: Vec<(String, u16)>,
    nlocals: u16,
    sigs: &'a HashMap<String, usize>,
    strings: &'a mut Vec<String>,
    patches: Vec<(usize, String)>,
}

impl<'a> FnCompiler<'a> {
    fn lookup(&self, n: &str) -> Result<u16, CompileError> {
        self.scopes.iter().rev().find(|(name, _)| name == n).map(|(_, s)| *s).ok_or_else(|| CompileError::Unbound(n.to_string()))
    }
    fn emit(&mut self, op: Op) -> usize {
        self.code.push(op);
        self.code.len() - 1
    }
    fn here(&self) -> u32 {
        self.code.len() as u32
    }
    fn compile(&mut self, e: &Ast) -> Result<(), CompileError> {
        match e {
            Ast::Int(n) => {
                self.emit(Op::Push(*n));
            }
            Ast::Bool(b) => {
                self.emit(Op::PushBool(*b));
            }
            Ast::Unit => {
                self.emit(Op::PushUnit);
            }
            Ast::Str(s) => {
                let idx = match self.strings.iter().position(|x| x == s) {
                    Some(i) => i,
                    None => {
                        self.strings.push(s.clone());
                        self.strings.len() - 1
                    }
                };
                self.emit(Op::Str(idx as u16));
            }
            Ast::Var(n) => {
                let s = self.lookup(n)?;
                self.emit(Op::Load(s));
            }
            Ast::Let(n, v, body) => {
                self.compile(v)?;
                let slot = self.nlocals;
                self.nlocals += 1;
                self.emit(Op::Store(slot));
                self.scopes.push((n.clone(), slot));
                self.compile(body)?;
                self.scopes.pop();
            }
            Ast::Set(n, v) => {
                self.compile(v)?;
                let s = self.lookup(n)?;
                self.emit(Op::Store(s));
                self.emit(Op::PushUnit);
            }
            Ast::Bin(op, a, b) => {
                self.compile(a)?;
                self.compile(b)?;
                let o = match op {
                    '+' => Op::Add,
                    '-' => Op::Sub,
                    '*' => Op::Mul,
                    '/' => Op::Div,
                    '%' => Op::Mod,
                    '<' => Op::Lt,
                    '=' => Op::Eq,
                    '>' => {
                        // a > b  ==  b < a: swap by recompiling
                        self.code.truncate(self.code.len());
                        self.emit(Op::Lt);
                        self.emit(Op::Not);
                        return Ok(());
                    }
                    c => panic!("bad op {}", c),
                };
                self.emit(o);
            }
            Ast::If(c, t, f) => {
                self.compile(c)?;
                let jz = self.emit(Op::Jz(0));
                self.compile(t)?;
                let jmp = self.emit(Op::Jmp(0));
                let else_at = self.here();
                self.code[jz] = Op::Jz(else_at);
                self.compile(f)?;
                let end = self.here();
                self.code[jmp] = Op::Jmp(end);
            }
            Ast::While(c, b) => {
                let top = self.here();
                self.compile(c)?;
                let jz = self.emit(Op::Jz(0));
                self.compile(b)?;
                self.emit(Op::Pop);
                self.emit(Op::Jmp(top));
                let end = self.here();
                self.code[jz] = Op::Jz(end);
                self.emit(Op::PushUnit);
            }
            Ast::Seq(es) => {
                if es.is_empty() {
                    self.emit(Op::PushUnit);
                }
                for (i, e) in es.iter().enumerate() {
                    self.compile(e)?;
                    if i + 1 < es.len() {
                        self.emit(Op::Pop);
                    }
                }
            }
            Ast::Call(name, args) => {
                let want = *self.sigs.get(name).ok_or_else(|| CompileError::UnknownFn(name.clone()))?;
                if want != args.len() {
                    return Err(CompileError::Arity { name: name.clone(), want, got: args.len() });
                }
                for a in args {
                    self.compile(a)?;
                }
                let at = self.emit(Op::Call(0, args.len() as u8));
                self.patches.push((at, name.clone()));
            }
            Ast::Cons(a, b) => {
                self.compile(a)?;
                self.compile(b)?;
                self.emit(Op::Cons);
            }
            Ast::Car(a) => {
                self.compile(a)?;
                self.emit(Op::Car);
            }
            Ast::Cdr(a) => {
                self.compile(a)?;
                self.emit(Op::Cdr);
            }
            Ast::Arr(xs) => {
                for x in xs {
                    self.compile(x)?;
                }
                self.emit(Op::NewArr(xs.len() as u16));
            }
            Ast::Index(a, i) => {
                self.compile(a)?;
                self.compile(i)?;
                self.emit(Op::ArrGet);
            }
            Ast::SetIndex(a, i, v) => {
                self.compile(a)?;
                self.compile(i)?;
                self.compile(v)?;
                self.emit(Op::ArrSet);
                self.emit(Op::PushUnit);
            }
            Ast::Len(a) => {
                self.compile(a)?;
                self.emit(Op::ArrLen);
            }
            Ast::Print(a) => {
                self.compile(a)?;
                self.emit(Op::Print);
                self.emit(Op::PushUnit);
            }
        }
        Ok(())
    }
}

fn compile_program(fns: &[FnProto]) -> Result<Compiled, CompileError> {
    let sigs: HashMap<String, usize> = fns.iter().map(|f| (f.name.clone(), f.params.len())).collect();
    let mut code = Vec::new();
    let mut strings = Vec::new();
    let mut entry = HashMap::new();
    let mut patches = Vec::new();
    for f in fns {
        let start = code.len() as u32;
        let mut fc = FnCompiler { code: &mut code, scopes: vec![], nlocals: 0, sigs: &sigs, strings: &mut strings, patches: vec![] };
        for p in &f.params {
            let s = fc.nlocals;
            fc.scopes.push((p.clone(), s));
            fc.nlocals += 1;
        }
        fc.compile(&f.body)?;
        fc.emit(Op::Ret);
        let nl = fc.nlocals;
        patches.extend(fc.patches.drain(..));
        entry.insert(f.name.clone(), (start, nl));
    }
    let names: Vec<String> = fns.iter().map(|f| f.name.clone()).collect();
    for (at, name) in patches {
        let idx = names.iter().position(|n| *n == name).unwrap();
        if let Op::Call(_, argc) = code[at] {
            code[at] = Op::Call(idx as u16, argc);
        }
    }
    let _ = names;
    Ok(Compiled { code, entry, strings })
}

struct Frame {
    ret_pc: usize,
    base: usize,
}

#[derive(Debug)]
enum VmError {
    Type(&'static str, usize),
    DivZero(usize),
    Bounds { idx: i64, len: usize },
    StackOverflow,
}

struct Vm<'p> {
    prog: &'p Compiled,
    fn_table: Vec<(u32, u16)>,
    stack: Vec<Val>,
    frames: Vec<Frame>,
    heap: Heap,
    out: String,
    steps: u64,
    gc_threshold: usize,
}

impl<'p> Vm<'p> {
    fn new(prog: &'p Compiled, order: &[String]) -> Self {
        let fn_table = order.iter().map(|n| prog.entry[n]).collect();
        Vm { prog, fn_table, stack: Vec::with_capacity(256), frames: vec![], heap: Heap::default(), out: String::new(), steps: 0, gc_threshold: 64 }
    }

    fn pop_int(&mut self, pc: usize) -> Result<i64, VmError> {
        match self.stack.pop() {
            Some(Val::Int(n)) => Ok(n),
            _ => Err(VmError::Type("int", pc)),
        }
    }

    fn pop_ref(&mut self, pc: usize) -> Result<HeapId, VmError> {
        match self.stack.pop() {
            Some(Val::Ref(h)) => Ok(h),
            _ => Err(VmError::Type("ref", pc)),
        }
    }

    fn maybe_gc(&mut self) {
        if self.heap.live() >= self.gc_threshold {
            let freed = self.heap.collect(self.stack.iter());
            let live = self.heap.live();
            if live * 2 > self.gc_threshold {
                self.gc_threshold *= 2;
            }
            let _ = freed;
        }
    }

    fn show(&self, v: Val, out: &mut String, depth: usize) {
        match v {
            Val::Int(n) => write!(out, "{}", n).unwrap(),
            Val::Bool(b) => write!(out, "{}", b).unwrap(),
            Val::Unit => out.push_str("()"),
            Val::Ref(h) => {
                if depth > 8 {
                    out.push_str("...");
                    return;
                }
                match self.heap.get(h) {
                    Obj::Pair(a, b) => {
                        out.push('(');
                        self.show(*a, out, depth + 1);
                        out.push_str(" . ");
                        self.show(*b, out, depth + 1);
                        out.push(')');
                    }
                    Obj::Array(xs) => {
                        out.push('[');
                        for (i, x) in xs.iter().enumerate() {
                            if i > 0 {
                                out.push(' ');
                            }
                            self.show(*x, out, depth + 1);
                        }
                        out.push(']');
                    }
                    Obj::Str(s) => out.push_str(s),
                }
            }
        }
    }

    fn run(&mut self, main_idx: usize) -> Result<Val, VmError> {
        let (start, nl) = self.fn_table[main_idx];
        let mut pc = start as usize;
        self.frames.push(Frame { ret_pc: usize::MAX, base: 0 });
        self.stack.resize(nl as usize, Val::Unit);
        loop {
            self.steps += 1;
            let op = self.prog.code[pc];
            pc += 1;
            let base = self.frames.last().unwrap().base;
            match op {
                Op::Push(n) => self.stack.push(Val::Int(n)),
                Op::PushBool(b) => self.stack.push(Val::Bool(b)),
                Op::PushUnit => self.stack.push(Val::Unit),
                Op::Load(s) => {
                    let v = self.stack[base + s as usize];
                    self.stack.push(v);
                }
                Op::Store(s) => {
                    let v = self.stack.pop().unwrap();
                    self.stack[base + s as usize] = v;
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Lt => {
                    let b = self.pop_int(pc)?;
                    let a = self.pop_int(pc)?;
                    let r = match op {
                        Op::Add => Val::Int(a.wrapping_add(b)),
                        Op::Sub => Val::Int(a.wrapping_sub(b)),
                        Op::Mul => Val::Int(a.wrapping_mul(b)),
                        Op::Div => Val::Int(a.checked_div(b).ok_or(VmError::DivZero(pc))?),
                        Op::Mod => Val::Int(a.checked_rem(b).ok_or(VmError::DivZero(pc))?),
                        Op::Lt => Val::Bool(a < b),
                        _ => unreachable!(),
                    };
                    self.stack.push(r);
                }
                Op::Eq => {
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    self.stack.push(Val::Bool(a == b));
                }
                Op::Not => match self.stack.pop() {
                    Some(Val::Bool(b)) => self.stack.push(Val::Bool(!b)),
                    _ => return Err(VmError::Type("bool", pc)),
                },
                Op::Jmp(t) => pc = t as usize,
                Op::Jz(t) => match self.stack.pop() {
                    Some(Val::Bool(false)) => pc = t as usize,
                    Some(Val::Bool(true)) => {}
                    _ => return Err(VmError::Type("bool", pc)),
                },
                Op::Call(fi, argc) => {
                    if self.frames.len() > 500 {
                        return Err(VmError::StackOverflow);
                    }
                    let (entry, nl) = self.fn_table[fi as usize];
                    let base = self.stack.len() - argc as usize;
                    for _ in argc as u16..nl {
                        self.stack.push(Val::Unit);
                    }
                    self.frames.push(Frame { ret_pc: pc, base });
                    pc = entry as usize;
                }
                Op::Ret => {
                    let v = self.stack.pop().unwrap();
                    let fr = self.frames.pop().unwrap();
                    self.stack.truncate(fr.base);
                    if fr.ret_pc == usize::MAX {
                        return Ok(v);
                    }
                    self.stack.push(v);
                    pc = fr.ret_pc;
                }
                Op::Pop => {
                    self.stack.pop();
                }
                Op::Cons => {
                    self.maybe_gc();
                    let b = self.stack.pop().unwrap();
                    let a = self.stack.pop().unwrap();
                    let h = self.heap.alloc(Obj::Pair(a, b));
                    self.stack.push(Val::Ref(h));
                }
                Op::Car | Op::Cdr => {
                    let h = self.pop_ref(pc)?;
                    match self.heap.get(h) {
                        Obj::Pair(a, b) => self.stack.push(if op == Op::Car { *a } else { *b }),
                        _ => return Err(VmError::Type("pair", pc)),
                    }
                }
                Op::NewArr(n) => {
                    self.maybe_gc();
                    let at = self.stack.len() - n as usize;
                    let xs: Vec<Val> = self.stack.drain(at..).collect();
                    let h = self.heap.alloc(Obj::Array(xs));
                    self.stack.push(Val::Ref(h));
                }
                Op::ArrGet => {
                    let i = self.pop_int(pc)?;
                    let h = self.pop_ref(pc)?;
                    match self.heap.get(h) {
                        Obj::Array(xs) => {
                            let v = *xs.get(i as usize).ok_or(VmError::Bounds { idx: i, len: xs.len() })?;
                            self.stack.push(v);
                        }
                        _ => return Err(VmError::Type("array", pc)),
                    }
                }
                Op::ArrSet => {
                    let v = self.stack.pop().unwrap();
                    let i = self.pop_int(pc)?;
                    let h = self.pop_ref(pc)?;
                    match self.heap.get_mut(h) {
                        Obj::Array(xs) => {
                            let len = xs.len();
                            *xs.get_mut(i as usize).ok_or(VmError::Bounds { idx: i, len })? = v;
                        }
                        _ => return Err(VmError::Type("array", pc)),
                    }
                }
                Op::ArrLen => {
                    let h = self.pop_ref(pc)?;
                    let n = match self.heap.get(h) {
                        Obj::Array(xs) => xs.len(),
                        Obj::Str(s) => s.len(),
                        Obj::Pair(..) => 2,
                    };
                    self.stack.push(Val::Int(n as i64));
                }
                Op::Str(i) => {
                    self.maybe_gc();
                    let h = self.heap.alloc(Obj::Str(self.prog.strings[i as usize].clone()));
                    self.stack.push(Val::Ref(h));
                }
                Op::Concat => unreachable!(),
                Op::Print => {
                    let v = self.stack.pop().unwrap();
                    let mut s = String::new();
                    self.show(v, &mut s, 0);
                    self.out.push_str(&s);
                    self.out.push('\n');
                }
                Op::Halt => return Ok(Val::Unit),
            }
        }
    }
}

// ---- tiny AST builders ----
fn int(n: i64) -> Ast {
    Ast::Int(n)
}
fn var(s: &str) -> Ast {
    Ast::Var(s.to_string())
}
fn bin(op: char, a: Ast, b: Ast) -> Ast {
    Ast::Bin(op, Box::new(a), Box::new(b))
}
fn let_(n: &str, v: Ast, body: Ast) -> Ast {
    Ast::Let(n.into(), Box::new(v), Box::new(body))
}
fn set(n: &str, v: Ast) -> Ast {
    Ast::Set(n.into(), Box::new(v))
}
fn if_(c: Ast, t: Ast, f: Ast) -> Ast {
    Ast::If(Box::new(c), Box::new(t), Box::new(f))
}
fn while_(c: Ast, b: Ast) -> Ast {
    Ast::While(Box::new(c), Box::new(b))
}
fn call(n: &str, args: Vec<Ast>) -> Ast {
    Ast::Call(n.into(), args)
}
fn seq(v: Vec<Ast>) -> Ast {
    Ast::Seq(v)
}
fn print(a: Ast) -> Ast {
    Ast::Print(Box::new(a))
}
fn func(name: &str, params: &[&str], body: Ast) -> FnProto {
    FnProto { name: name.into(), params: params.iter().map(|s| s.to_string()).collect(), body }
}

fn programs() -> Vec<(&'static str, Vec<FnProto>)> {
    let mut v = Vec::new();
    // 1. factorial / fib
    v.push((
        "arith",
        vec![
            func("fact", &["n"], if_(bin('<', var("n"), int(2)), int(1), bin('*', var("n"), call("fact", vec![bin('-', var("n"), int(1))])))),
            func("fib", &["n"], if_(bin('<', var("n"), int(2)), var("n"), bin('+', call("fib", vec![bin('-', var("n"), int(1))]), call("fib", vec![bin('-', var("n"), int(2))])))),
            func(
                "main",
                &[],
                seq(vec![
                    print(call("fact", vec![int(20)])),
                    print(call("fib", vec![int(20)])),
                    let_("i", int(0), let_("s", int(0), seq(vec![
                        while_(bin('<', var("i"), int(1000)), seq(vec![set("s", bin('+', var("s"), bin('%', bin('*', var("i"), var("i")), int(7)))), set("i", bin('+', var("i"), int(1)))])),
                        print(var("s")),
                    ]))),
                    int(0),
                ]),
            ),
        ],
    ));
    // 2. linked lists with garbage
    v.push((
        "lists",
        vec![
            func("range", &["a", "b"], if_(bin('<', var("a"), var("b")), Ast::Cons(Box::new(var("a")), Box::new(call("range", vec![bin('+', var("a"), int(1)), var("b")]))), Ast::Unit)),
            func("sum", &["l"], if_(bin('=', var("l"), Ast::Unit), int(0), bin('+', Ast::Car(Box::new(var("l"))), call("sum", vec![Ast::Cdr(Box::new(var("l")))])))),
            func("rev", &["l", "acc"], if_(bin('=', var("l"), Ast::Unit), var("acc"), call("rev", vec![Ast::Cdr(Box::new(var("l"))), Ast::Cons(Box::new(Ast::Car(Box::new(var("l")))), Box::new(var("acc")))]))),
            func(
                "main",
                &[],
                let_("k", int(0), let_("tot", int(0), seq(vec![
                    while_(bin('<', var("k"), int(60)), seq(vec![
                        set("tot", bin('+', var("tot"), call("sum", vec![call("rev", vec![call("range", vec![int(0), var("k")]), Ast::Unit])]))),
                        set("k", bin('+', var("k"), int(1))),
                    ])),
                    print(var("tot")),
                    print(call("rev", vec![call("range", vec![int(0), int(6)]), Ast::Unit])),
                    int(1),
                ]))),
            ),
        ],
    ));
    // 3. arrays: sieve + bubble sort
    v.push((
        "arrays",
        vec![
            func("mkarr", &["n"], let_("a", Ast::Arr(vec![]), let_("i", int(0), seq(vec![
                // build via cons then copy: arrays are fixed-size, so build list of n zeros
                set("a", Ast::Arr((0..30).map(|_| int(0)).collect())),
                var("a"),
            ])))),
            func("sieve", &[], let_("a", call("mkarr", vec![int(30)]), let_("i", int(2), seq(vec![
                while_(bin('<', var("i"), int(30)), seq(vec![
                    if_(bin('=', Ast::Index(Box::new(var("a")), Box::new(var("i"))), int(0)),
                        let_("j", bin('*', var("i"), var("i")), while_(bin('<', var("j"), int(30)), seq(vec![
                            Ast::SetIndex(Box::new(var("a")), Box::new(var("j")), Box::new(int(1))),
                            set("j", bin('+', var("j"), var("i"))),
                        ]))),
                        Ast::Unit),
                    set("i", bin('+', var("i"), int(1))),
                ])),
                var("a"),
            ])))),
            func("bsort", &["a"], let_("n", Ast::Len(Box::new(var("a"))), let_("i", int(0), seq(vec![
                while_(bin('<', var("i"), var("n")), let_("j", int(0), seq(vec![
                    while_(bin('<', var("j"), bin('-', bin('-', var("n"), var("i")), int(1))), seq(vec![
                        if_(bin('<', Ast::Index(Box::new(var("a")), Box::new(bin('+', var("j"), int(1)))), Ast::Index(Box::new(var("a")), Box::new(var("j")))),
                            let_("t", Ast::Index(Box::new(var("a")), Box::new(var("j"))), seq(vec![
                                Ast::SetIndex(Box::new(var("a")), Box::new(var("j")), Box::new(Ast::Index(Box::new(var("a")), Box::new(bin('+', var("j"), int(1))))))
                                , Ast::SetIndex(Box::new(var("a")), Box::new(bin('+', var("j"), int(1))), Box::new(var("t"))),
                            ])),
                            Ast::Unit),
                        set("j", bin('+', var("j"), int(1))),
                    ])),
                    set("i", bin('+', var("i"), int(1))),
                ]))),
                var("a"),
            ])))),
            func("main", &[], seq(vec![
                print(call("sieve", vec![])),
                print(call("bsort", vec![Ast::Arr([9, 3, 7, 1, 8, 2, 6, 5, 4, 0].iter().map(|&x| int(x)).collect())])),
                print(Ast::Str("done".into())),
                print(Ast::Len(Box::new(Ast::Str("hello".into())))),
                int(2),
            ])),
        ],
    ));
    // 4. errors
    v.push(("divzero", vec![func("main", &[], bin('/', int(1), bin('-', int(3), int(3))))]));
    v.push(("bounds", vec![func("main", &[], Ast::Index(Box::new(Ast::Arr(vec![int(1)])), Box::new(int(5))))]));
    v.push(("unbound", vec![func("main", &[], var("zz"))]));
    v.push(("arity", vec![func("f", &["a"], var("a")), func("main", &[], call("f", vec![]))]));
    v.push(("overflow", vec![func("f", &["a"], call("f", vec![var("a")])), func("main", &[], call("f", vec![int(1)]))]));
    v.push(("typeerr", vec![func("main", &[], bin('+', int(1), Ast::Bool(true)))]));
    v
}

fn disasm(code: &[Op], limit: usize) -> String {
    let mut s = String::new();
    for (i, op) in code.iter().take(limit).enumerate() {
        let _ = writeln!(s, "{:04} {:?}", i, op);
    }
    s
}

fn main() {
    let mut summary: BTreeMap<&str, String> = BTreeMap::new();
    for (name, fns) in programs() {
        println!("--- {} ---", name);
        let order: Vec<String> = fns.iter().map(|f| f.name.clone()).collect();
        let prog = match compile_program(&fns) {
            Ok(p) => p,
            Err(e) => {
                println!("compile error: {}", e);
                summary.insert(name, "compile-error".into());
                continue;
            }
        };
        if name == "arith" {
            print!("{}", disasm(&prog.code, 12));
        }
        let main_idx = order.iter().position(|n| n == "main").unwrap();
        let mut vm = Vm::new(&prog, &order);
        let r = vm.run(main_idx);
        print!("{}", vm.out);
        let st = match r {
            Ok(v) => {
                let mut s = String::new();
                vm.show(v, &mut s, 0);
                format!("ok {} steps={} allocs={} gcs={} freed={} live={}", s, vm.steps, vm.heap.allocs, vm.heap.collections, vm.heap.freed_total, vm.heap.live())
            }
            Err(e) => format!("vm error: {:?}", e),
        };
        println!("{}", st);
        summary.insert(name, st.split_whitespace().next().unwrap_or("").to_string());
    }
    println!("{:?}", summary);
}
