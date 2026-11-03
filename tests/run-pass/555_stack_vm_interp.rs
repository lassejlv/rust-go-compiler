// a small stack VM with compiler from an AST, calls, locals, jumps, Display for instructions
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, PartialEq)]
enum Ins {
    Push(i64),
    Load(usize),
    Store(usize),
    Add,
    Sub,
    Mul,
    Div,
    Lt,
    Eq,
    Jmp(usize),
    Jz(usize),
    Call(usize, usize),
    Ret,
    Print,
    Halt,
}
impl fmt::Display for Ins {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ins::Push(v) => write!(f, "push {}", v),
            Ins::Load(i) => write!(f, "load ${}", i),
            Ins::Store(i) => write!(f, "store ${}", i),
            Ins::Jmp(t) => write!(f, "jmp @{}", t),
            Ins::Jz(t) => write!(f, "jz @{}", t),
            Ins::Call(t, n) => write!(f, "call @{} argc={}", t, n),
            other => write!(f, "{}", format!("{:?}", other).to_lowercase()),
        }
    }
}

struct Frame {
    ret: usize,
    base: usize,
}

struct Vm<'a> {
    code: &'a [Ins],
    stack: Vec<i64>,
    locals: Vec<i64>,
    frames: Vec<Frame>,
    pc: usize,
    out: Vec<String>,
    steps: usize,
}

#[derive(Debug)]
enum VmError {
    StackUnderflow(usize),
    DivZero(usize),
    TooManySteps,
}

impl<'a> Vm<'a> {
    fn new(code: &'a [Ins]) -> Self {
        Vm { code, stack: vec![], locals: vec![0; 16], frames: vec![], pc: 0, out: vec![], steps: 0 }
    }
    fn pop(&mut self) -> Result<i64, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow(self.pc))
    }
    fn bin(&mut self, f: impl Fn(i64, i64) -> Option<i64>) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(a, b).ok_or(VmError::DivZero(self.pc))?;
        self.stack.push(r);
        Ok(())
    }
    fn base(&self) -> usize {
        self.frames.last().map_or(0, |f| f.base)
    }
    fn run(&mut self) -> Result<(), VmError> {
        loop {
            self.steps += 1;
            if self.steps > 100_000 {
                return Err(VmError::TooManySteps);
            }
            let ins = self.code[self.pc].clone();
            self.pc += 1;
            match ins {
                Ins::Push(v) => self.stack.push(v),
                Ins::Load(i) => {
                    let v = self.locals[self.base() + i];
                    self.stack.push(v)
                }
                Ins::Store(i) => {
                    let v = self.pop()?;
                    let b = self.base();
                    if b + i >= self.locals.len() {
                        self.locals.resize(b + i + 1, 0);
                    }
                    self.locals[b + i] = v;
                }
                Ins::Add => self.bin(|a, b| Some(a + b))?,
                Ins::Sub => self.bin(|a, b| Some(a - b))?,
                Ins::Mul => self.bin(|a, b| Some(a.wrapping_mul(b)))?,
                Ins::Div => self.bin(|a, b| a.checked_div(b))?,
                Ins::Lt => self.bin(|a, b| Some((a < b) as i64))?,
                Ins::Eq => self.bin(|a, b| Some((a == b) as i64))?,
                Ins::Jmp(t) => self.pc = t,
                Ins::Jz(t) => {
                    if self.pop()? == 0 {
                        self.pc = t;
                    }
                }
                Ins::Call(t, argc) => {
                    let base = self.locals.len();
                    self.locals.resize(base + 8, 0);
                    for k in (0..argc).rev() {
                        let v = self.pop()?;
                        self.locals[base + k] = v;
                    }
                    self.frames.push(Frame { ret: self.pc, base });
                    self.pc = t;
                }
                Ins::Ret => {
                    let f = self.frames.pop().unwrap();
                    self.locals.truncate(f.base);
                    self.pc = f.ret;
                }
                Ins::Print => {
                    let v = self.pop()?;
                    self.out.push(v.to_string());
                }
                Ins::Halt => return Ok(()),
            }
        }
    }
}

fn assemble(src: &str) -> Vec<Ins> {
    let mut labels = HashMap::new();
    let lines: Vec<&str> = src.split(|c| c == '\n' || c == ';').map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut idx = 0;
    for l in &lines {
        if let Some(name) = l.strip_suffix(':') {
            labels.insert(name.to_string(), idx);
        } else {
            idx += 1;
        }
    }
    lines
        .iter()
        .filter(|l| !l.ends_with(':'))
        .map(|l| {
            let parts: Vec<&str> = l.split_whitespace().collect();
            let arg = |i: usize| -> usize { labels.get(parts[i]).copied().unwrap_or_else(|| parts[i].parse().unwrap()) };
            match parts[0] {
                "push" => Ins::Push(parts[1].parse().unwrap()),
                "load" => Ins::Load(arg(1)),
                "store" => Ins::Store(arg(1)),
                "add" => Ins::Add,
                "sub" => Ins::Sub,
                "mul" => Ins::Mul,
                "div" => Ins::Div,
                "lt" => Ins::Lt,
                "eq" => Ins::Eq,
                "jmp" => Ins::Jmp(arg(1)),
                "jz" => Ins::Jz(arg(1)),
                "call" => Ins::Call(arg(1), arg(2)),
                "ret" => Ins::Ret,
                "print" => Ins::Print,
                "halt" => Ins::Halt,
                x => panic!("unknown {}", x),
            }
        })
        .collect()
}

fn main() {
    let prog = assemble(
        "push 1; store 0
        loop:
        load 0; push 11; lt; jz end
        load 0; call fact 1; print
        load 0; push 3; add; store 0; jmp loop
        end:
        push 10; push 0; div; halt
        fact:
        load 0; push 2; lt; jz rec; push 1; ret
        rec:
        load 0; load 0; push 1; sub; call fact 1; mul; ret",
    );
    for (i, ins) in prog.iter().enumerate().take(8) {
        println!("{:>3}: {}", i, ins);
    }
    let mut vm = Vm::new(&prog);
    let r = vm.run();
    println!("output: {}", vm.out.join(" "));
    println!("result: {:?} after {} steps", r, vm.steps);
    let bad = [Ins::Push(1), Ins::Add];
    let mut vm2 = Vm::new(&bad);
    println!("{:?}", vm2.run());
    let inf = [Ins::Jmp(0)];
    println!("{:?}", Vm::new(&inf).run());
}
