use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Op {
    Push(i64),
    Pop,
    Add,
    Sub,
    Mul,
    Div,
    Mod,
    Neg,
    Dup,
    Swap,
    Over,
    Jmp(usize),
    Jz(usize),
    Jnz(usize),
    Load(u8),
    Store(u8),
    Call(usize),
    Ret,
    Print,
    Lt,
    Eq,
    Shl,
    Shr,
    And,
    Or,
    Xor,
    Halt,
}

#[derive(Debug, PartialEq)]
enum VmError {
    StackUnderflow(usize),
    DivByZero { pc: usize },
    BadJump(usize),
    Overflow(&'static str),
    StepLimit,
}

impl fmt::Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmError::StackUnderflow(pc) => write!(f, "stack underflow at pc={}", pc),
            VmError::DivByZero { pc } => write!(f, "division by zero at pc={pc}"),
            VmError::BadJump(t) => write!(f, "bad jump target {:#06x}", t),
            VmError::Overflow(op) => write!(f, "overflow in {}", op),
            VmError::StepLimit => f.write_str("step limit"),
        }
    }
}

struct Vm<'p> {
    prog: &'p [Op],
    stack: Vec<i64>,
    frames: Vec<(usize, [i64; 4])>,
    locals: [i64; 4],
    pc: usize,
    out: Vec<String>,
    steps: u32,
}

impl<'p> Vm<'p> {
    fn new(prog: &'p [Op]) -> Self {
        Vm { prog, stack: Vec::with_capacity(16), frames: vec![], locals: [0; 4], pc: 0, out: vec![], steps: 0 }
    }

    fn pop(&mut self) -> Result<i64, VmError> {
        self.stack.pop().ok_or(VmError::StackUnderflow(self.pc))
    }

    fn bin(&mut self, name: &'static str, f: impl Fn(i64, i64) -> Option<i64>) -> Result<(), VmError> {
        let b = self.pop()?;
        let a = self.pop()?;
        let r = f(a, b).ok_or(VmError::Overflow(name))?;
        self.stack.push(r);
        Ok(())
    }

    fn jump(&mut self, t: usize) -> Result<(), VmError> {
        if t >= self.prog.len() {
            return Err(VmError::BadJump(t));
        }
        self.pc = t;
        Ok(())
    }

    fn run(&mut self) -> Result<Option<i64>, VmError> {
        loop {
            self.steps += 1;
            if self.steps > 10_000 {
                return Err(VmError::StepLimit);
            }
            let op = *self.prog.get(self.pc).ok_or(VmError::BadJump(self.pc))?;
            self.pc += 1;
            match op {
                Op::Push(v) => self.stack.push(v),
                Op::Pop => {
                    self.pop()?;
                }
                Op::Add => self.bin("add", i64::checked_add)?,
                Op::Sub => self.bin("sub", i64::checked_sub)?,
                Op::Mul => self.bin("mul", |a, b| a.checked_mul(b))?,
                Op::Div | Op::Mod => {
                    let b = self.pop()?;
                    let a = self.pop()?;
                    if b == 0 {
                        return Err(VmError::DivByZero { pc: self.pc - 1 });
                    }
                    self.stack.push(if op == Op::Div { a.wrapping_div(b) } else { a.rem_euclid(b) });
                }
                Op::Neg => {
                    let a = self.pop()?;
                    self.stack.push(a.checked_neg().ok_or(VmError::Overflow("neg"))?);
                }
                Op::Dup => {
                    let a = *self.stack.last().ok_or(VmError::StackUnderflow(self.pc))?;
                    self.stack.push(a);
                }
                Op::Swap => {
                    let n = self.stack.len();
                    if n < 2 {
                        return Err(VmError::StackUnderflow(self.pc));
                    }
                    self.stack.swap(n - 1, n - 2);
                }
                Op::Over => {
                    let n = self.stack.len();
                    let v = *self.stack.get(n.wrapping_sub(2)).ok_or(VmError::StackUnderflow(self.pc))?;
                    self.stack.push(v);
                }
                Op::Jmp(t) => self.jump(t)?,
                Op::Jz(t) => {
                    if self.pop()? == 0 {
                        self.jump(t)?
                    }
                }
                Op::Jnz(t) => {
                    if self.pop()? != 0 {
                        self.jump(t)?
                    }
                }
                Op::Load(i) => self.stack.push(self.locals[i as usize]),
                Op::Store(i) => self.locals[i as usize] = self.pop()?,
                Op::Call(t) => {
                    self.frames.push((self.pc, self.locals));
                    self.locals = [0; 4];
                    self.jump(t)?;
                }
                Op::Ret => match self.frames.pop() {
                    Some((ret, locals)) => {
                        self.pc = ret;
                        self.locals = locals;
                    }
                    None => return Ok(self.stack.last().copied()),
                },
                Op::Print => {
                    let v = self.pop()?;
                    self.out.push(v.to_string());
                }
                Op::Lt => self.bin("lt", |a, b| Some((a < b) as i64))?,
                Op::Eq => self.bin("eq", |a, b| Some(i64::from(a == b)))?,
                Op::Shl => self.bin("shl", |a, b| a.checked_shl(u32::try_from(b).ok()?))?,
                Op::Shr => self.bin("shr", |a, b| Some(a >> (b & 63)))?,
                Op::And => self.bin("and", |a, b| Some(a & b))?,
                Op::Or => self.bin("or", |a, b| Some(a | b))?,
                Op::Xor => self.bin("xor", |a, b| Some(a ^ b))?,
                Op::Halt => return Ok(self.stack.last().copied()),
            }
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
    let target = |s: &str| -> usize { labels.get(s).copied().unwrap_or_else(|| s.parse().unwrap()) };
    src.lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.ends_with(':'))
        .map(|line| {
            let mut parts = line.split_whitespace();
            let op = parts.next().unwrap();
            let arg = parts.next();
            match (op.to_ascii_lowercase().as_str(), arg) {
                ("push", Some(a)) => Op::Push(a.parse().unwrap()),
                ("pop", _) => Op::Pop,
                ("add", _) => Op::Add,
                ("sub", _) => Op::Sub,
                ("mul", _) => Op::Mul,
                ("div", _) => Op::Div,
                ("mod", _) => Op::Mod,
                ("neg", _) => Op::Neg,
                ("dup", _) => Op::Dup,
                ("swap", _) => Op::Swap,
                ("over", _) => Op::Over,
                ("jmp", Some(a)) => Op::Jmp(target(a)),
                ("jz", Some(a)) => Op::Jz(target(a)),
                ("jnz", Some(a)) => Op::Jnz(target(a)),
                ("load", Some(a)) => Op::Load(a.parse().unwrap()),
                ("store", Some(a)) => Op::Store(a.parse().unwrap()),
                ("call", Some(a)) => Op::Call(target(a)),
                ("ret", _) => Op::Ret,
                ("print", _) => Op::Print,
                ("lt", _) => Op::Lt,
                ("eq", _) => Op::Eq,
                ("shl", _) => Op::Shl,
                ("shr", _) => Op::Shr,
                ("and", _) => Op::And,
                ("or", _) => Op::Or,
                ("xor", _) => Op::Xor,
                ("halt", _) => Op::Halt,
                (o, a) => panic!("bad op {} {:?}", o, a),
            }
        })
        .collect()
}

fn main() {
    let fact = "
        push 1
        store 0
        push 20
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
        dup
        print
        halt
    ";
    let prog = assemble(fact);
    let mut vm = Vm::new(&prog);
    println!("{:?} {:?} steps={}", vm.run(), vm.out, vm.steps);

    let fib = "
        push 0
        store 0
        push 1
        store 1
        push 15
        store 2
    again:
        load 0
        print
        load 0
        load 1
        dup
        store 0
        add
        store 1
        load 2
        push 1
        sub
        dup
        store 2
        jnz again
        push 7
        call sq
        print
        halt
    sq:
        dup
        mul
        ret
    ";
    let prog = assemble(fib);
    let mut vm = Vm::new(&prog);
    let r = vm.run();
    println!("{:?} {}", r, vm.out.join(","));

    let bad = [
        vec![Op::Push(1), Op::Push(0), Op::Div],
        vec![Op::Add],
        vec![Op::Push(i64::MAX), Op::Push(1), Op::Add],
        vec![Op::Push(i64::MIN), Op::Neg],
        vec![Op::Jmp(99)],
        vec![Op::Jmp(0)],
        vec![Op::Push(1), Op::Push(64), Op::Shl],
        vec![Op::Push(-7), Op::Push(3), Op::Mod, Op::Push(-7), Op::Push(2), Op::Div, Op::Push(-1), Op::Push(3), Op::Shr, Op::Halt],
        vec![Op::Push(6), Op::Push(3), Op::Xor, Op::Push(12), Op::Or, Op::Push(10), Op::And, Op::Push(1), Op::Over, Op::Swap, Op::Lt, Op::Ret],
    ];
    for p in bad.iter() {
        let mut vm = Vm::new(p);
        match vm.run() {
            Ok(v) => println!("ok {:?} stack={:?}", v, vm.stack),
            Err(e) => println!("error: {} ({:?})", e, e),
        }
    }
}
