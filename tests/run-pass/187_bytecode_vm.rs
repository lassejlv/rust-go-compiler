use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Push(i64),
    Pop,
    Dup,
    Swap,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Eq,
    Lt,
    Not,
    Jmp(usize),
    Jz(usize),
    Load(u8),
    Store(u8),
    Call(usize),
    Ret,
    Print,
    Halt,
}

#[derive(Debug)]
enum VmError {
    StackUnderflow(usize),
    DivByZero(usize),
    BadJump(usize),
    StepLimit,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmError::StackUnderflow(pc) => write!(f, "stack underflow at pc={}", pc),
            VmError::DivByZero(pc) => write!(f, "division by zero at pc={}", pc),
            VmError::BadJump(pc) => write!(f, "bad jump at pc={}", pc),
            VmError::StepLimit => write!(f, "step limit exceeded"),
        }
    }
}

struct Vm<'a> {
    code: &'a [Op],
    stack: Vec<i64>,
    frames: Vec<(usize, [i64; 4])>,
    locals: [i64; 4],
    pc: usize,
    output: Vec<i64>,
    steps: usize,
}

impl<'a> Vm<'a> {
    fn new(code: &'a [Op]) -> Self {
        Vm { code, stack: Vec::new(), frames: Vec::new(), locals: [0; 4], pc: 0, output: Vec::new(), steps: 0 }
    }
    fn pop(&mut self) -> Result<i64, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow(self.pc))
    }
    fn run(&mut self) -> Result<(), VmError> {
        loop {
            self.steps += 1;
            if self.steps > 100_000 {
                return Err(VmError::StepLimit);
            }
            let op = *self.code.get(self.pc).ok_or(VmError::BadJump(self.pc))?;
            let mut next = self.pc + 1;
            match op {
                Op::Push(v) => self.stack.push(v),
                Op::Pop => {
                    self.pop()?;
                }
                Op::Dup => {
                    let v = self.pop()?;
                    self.stack.push(v);
                    self.stack.push(v);
                }
                Op::Swap => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    self.stack.push(b);
                    self.stack.push(a);
                }
                Op::Add | Op::Sub | Op::Mul | Op::Div | Op::Mod | Op::Eq | Op::Lt => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    let r = match op {
                        Op::Add => a.wrapping_add(b),
                        Op::Sub => a - b,
                        Op::Mul => a.wrapping_mul(b),
                        Op::Div | Op::Mod if b == 0 => return Err(VmError::DivByZero(self.pc)),
                        Op::Div => a / b,
                        Op::Mod => a % b,
                        Op::Eq => (a == b) as i64,
                        Op::Lt => (a < b) as i64,
                        _ => unreachable!(),
                    };
                    self.stack.push(r);
                }
                Op::Not => {
                    let v = self.pop()?;
                    self.stack.push((v == 0) as i64);
                }
                Op::Jmp(t) => next = t,
                Op::Jz(t) => {
                    if self.pop()? == 0 {
                        next = t;
                    }
                }
                Op::Load(i) => self.stack.push(self.locals[i as usize]),
                Op::Store(i) => self.locals[i as usize] = self.pop()?,
                Op::Call(t) => {
                    self.frames.push((next, self.locals));
                    self.locals = [0; 4];
                    next = t;
                }
                Op::Ret => match self.frames.pop() {
                    Some((ret, saved)) => {
                        self.locals = saved;
                        next = ret;
                    }
                    None => return Ok(()),
                },
                Op::Print => {
                    let v = self.pop()?;
                    self.output.push(v);
                }
                Op::Halt => return Ok(()),
            }
            self.pc = next;
        }
    }
}

fn assemble(src: &str) -> Vec<Op> {
    let mut labels = HashMap::new();
    let mut idx = 0;
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty()) {
        if let Some(l) = line.strip_suffix(':') {
            labels.insert(l.to_string(), idx);
        } else {
            idx += 1;
        }
    }
    let mut code = Vec::new();
    for line in src.lines().map(str::trim).filter(|l| !l.is_empty() && !l.ends_with(':')) {
        let mut parts = line.split_whitespace();
        let name = parts.next().unwrap();
        let arg = parts.next();
        let target = |a: Option<&str>| -> usize { labels[a.unwrap()] };
        let num = |a: Option<&str>| -> i64 { a.unwrap().parse().unwrap() };
        code.push(match name {
            "push" => Op::Push(num(arg)),
            "pop" => Op::Pop,
            "dup" => Op::Dup,
            "swap" => Op::Swap,
            "add" => Op::Add,
            "sub" => Op::Sub,
            "mul" => Op::Mul,
            "div" => Op::Div,
            "mod" => Op::Mod,
            "eq" => Op::Eq,
            "lt" => Op::Lt,
            "not" => Op::Not,
            "jmp" => Op::Jmp(target(arg)),
            "jz" => Op::Jz(target(arg)),
            "load" => Op::Load(num(arg) as u8),
            "store" => Op::Store(num(arg) as u8),
            "call" => Op::Call(target(arg)),
            "ret" => Op::Ret,
            "print" => Op::Print,
            "halt" => Op::Halt,
            other => panic!("unknown op {}", other),
        });
    }
    code
}

const FACTORIAL: &str = "
    push 1
    store 0
    push 10
    store 1
loop:
    load 1
    jz done
    load 0
    load 1
    mul
    store 0
    load 1
    push 1
    sub
    store 1
    jmp loop
done:
    load 0
    print
    halt
";

const FIB_CALLS: &str = "
    push 0
    store 2
next:
    load 2
    push 12
    lt
    jz end
    load 2
    call fib
    print
    load 2
    push 1
    add
    store 2
    jmp next
end:
    halt
fib:
    store 0
    push 0
    store 1
    push 1
    store 2
floop:
    load 0
    jz fdone
    load 1
    load 2
    dup
    store 1
    add
    store 2
    load 0
    push 1
    sub
    store 0
    jmp floop
fdone:
    load 1
    ret
";

fn main() {
    let programs = [
        ("factorial", assemble(FACTORIAL)),
        ("fib", assemble(FIB_CALLS)),
        ("arith", vec![Op::Push(7), Op::Push(3), Op::Swap, Op::Sub, Op::Dup, Op::Mul, Op::Print, Op::Push(17), Op::Push(5), Op::Mod, Op::Print, Op::Halt]),
        ("underflow", vec![Op::Push(1), Op::Add, Op::Halt]),
        ("divzero", vec![Op::Push(1), Op::Push(0), Op::Div, Op::Halt]),
        ("infinite", vec![Op::Jmp(0)]),
        ("badjump", vec![Op::Jmp(99)]),
    ];
    for (name, code) in programs.iter() {
        let mut vm = Vm::new(code);
        let res = vm.run();
        match res {
            Ok(()) => println!("{:<10} ok   output={:?} steps={} stack={:?}", name, vm.output, vm.steps, vm.stack),
            Err(e) => println!("{:<10} err  {} (output={:?})", name, e, vm.output),
        }
    }
    let code = assemble(FACTORIAL);
    println!("factorial program has {} ops; first {:?}, last {:?}", code.len(), code[0], code[code.len() - 1]);
}
