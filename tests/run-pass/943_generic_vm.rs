// A small stack VM generic over its value type, with a trait describing the value domain.
use std::collections::BTreeMap;
use std::fmt::{self, Debug, Display};

trait Value: Clone + Debug + Display + PartialEq {
    fn from_int(i: i64) -> Self;
    fn truthy(&self) -> bool;
    fn binop(op: Op, a: &Self, b: &Self) -> Result<Self, VmError>;
    fn type_name(&self) -> &'static str;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum Op { Add, Sub, Mul, Div, Lt, Eq }

#[derive(Debug, Clone, PartialEq)]
enum VmError { StackUnderflow(usize), DivZero, Type(String), BadJump(usize), Unbound(String), StepLimit }

impl Display for VmError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            VmError::StackUnderflow(pc) => write!(f, "stack underflow at pc={}", pc),
            VmError::DivZero => write!(f, "division by zero"),
            VmError::Type(s) => write!(f, "type error: {}", s),
            VmError::BadJump(t) => write!(f, "bad jump target {}", t),
            VmError::Unbound(n) => write!(f, "unbound variable {}", n),
            VmError::StepLimit => write!(f, "step limit exceeded"),
        }
    }
}

#[derive(Debug, Clone)]
enum Instr<V> { Push(V), Pop, Dup, Swap, Bin(Op), Load(String), Store(String), Jmp(usize), Jz(usize), Print, Halt }

impl Value for i64 {
    fn from_int(i: i64) -> Self { i }
    fn truthy(&self) -> bool { *self != 0 }
    fn binop(op: Op, a: &i64, b: &i64) -> Result<i64, VmError> {
        Ok(match op {
            Op::Add => a.wrapping_add(*b),
            Op::Sub => a.wrapping_sub(*b),
            Op::Mul => a.wrapping_mul(*b),
            Op::Div => { if *b == 0 { return Err(VmError::DivZero); } a / b }
            Op::Lt => (a < b) as i64,
            Op::Eq => (a == b) as i64,
        })
    }
    fn type_name(&self) -> &'static str { "int" }
}

#[derive(Debug, Clone, PartialEq)]
enum Dyn { I(i64), F(f64), S(String), B(bool) }
impl Display for Dyn {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { Dyn::I(i) => write!(f, "{}", i), Dyn::F(x) => write!(f, "{:.2}", x), Dyn::S(s) => write!(f, "{}", s), Dyn::B(b) => write!(f, "{}", b) }
    }
}
impl Value for Dyn {
    fn from_int(i: i64) -> Self { Dyn::I(i) }
    fn truthy(&self) -> bool { match self { Dyn::I(i) => *i != 0, Dyn::F(f) => *f != 0.0, Dyn::S(s) => !s.is_empty(), Dyn::B(b) => *b } }
    fn type_name(&self) -> &'static str { match self { Dyn::I(_) => "int", Dyn::F(_) => "float", Dyn::S(_) => "str", Dyn::B(_) => "bool" } }
    fn binop(op: Op, a: &Dyn, b: &Dyn) -> Result<Dyn, VmError> {
        use Dyn::*;
        match (op, a, b) {
            (_, I(x), I(y)) => match op {
                Op::Lt => Ok(B(x < y)),
                Op::Eq => Ok(B(x == y)),
                _ => i64::binop(op, x, y).map(I),
            },
            (Op::Add, S(x), S(y)) => Ok(S(format!("{}{}", x, y))),
            (Op::Mul, S(x), I(n)) => Ok(S(x.repeat(*n as usize))),
            (Op::Eq, x, y) => Ok(B(x == y)),
            (_, F(_), _) | (_, _, F(_)) => {
                let fx = match a { F(v) => *v, I(v) => *v as f64, _ => return Err(VmError::Type(format!("{} in float op", a.type_name()))) };
                let fy = match b { F(v) => *v, I(v) => *v as f64, _ => return Err(VmError::Type(format!("{} in float op", b.type_name()))) };
                match op {
                    Op::Add => Ok(F(fx + fy)), Op::Sub => Ok(F(fx - fy)), Op::Mul => Ok(F(fx * fy)),
                    Op::Div => if fy == 0.0 { Err(VmError::DivZero) } else { Ok(F(fx / fy)) },
                    Op::Lt => Ok(B(fx < fy)), Op::Eq => Ok(B(fx == fy)),
                }
            }
            _ => Err(VmError::Type(format!("{:?} on {} and {}", op, a.type_name(), b.type_name()))),
        }
    }
}

struct Vm<V: Value> {
    stack: Vec<V>,
    vars: BTreeMap<String, V>,
    out: Vec<String>,
    steps: usize,
}

impl<V: Value> Vm<V> {
    fn new() -> Self { Vm { stack: Vec::new(), vars: BTreeMap::new(), out: Vec::new(), steps: 0 } }

    fn pop(&mut self, pc: usize) -> Result<V, VmError> { self.stack.pop().ok_or(VmError::StackUnderflow(pc)) }

    fn run(&mut self, prog: &[Instr<V>], limit: usize) -> Result<Option<V>, VmError> {
        let mut pc = 0;
        while pc < prog.len() {
            self.steps += 1;
            if self.steps > limit { return Err(VmError::StepLimit); }
            let mut next = pc + 1;
            match &prog[pc] {
                Instr::Push(v) => self.stack.push(v.clone()),
                Instr::Pop => { self.pop(pc)?; }
                Instr::Dup => { let v = self.pop(pc)?; self.stack.push(v.clone()); self.stack.push(v); }
                Instr::Swap => { let a = self.pop(pc)?; let b = self.pop(pc)?; self.stack.push(a); self.stack.push(b); }
                Instr::Bin(op) => { let b = self.pop(pc)?; let a = self.pop(pc)?; self.stack.push(V::binop(*op, &a, &b)?); }
                Instr::Load(n) => { let v = self.vars.get(n).cloned().ok_or_else(|| VmError::Unbound(n.clone()))?; self.stack.push(v); }
                Instr::Store(n) => { let v = self.pop(pc)?; self.vars.insert(n.clone(), v); }
                Instr::Jmp(t) => next = *t,
                Instr::Jz(t) => { if !self.pop(pc)?.truthy() { next = *t; } }
                Instr::Print => { let v = self.pop(pc)?; self.out.push(v.to_string()); }
                Instr::Halt => return Ok(self.stack.last().cloned()),
            }
            if next > prog.len() { return Err(VmError::BadJump(next)); }
            pc = next;
        }
        Ok(self.stack.last().cloned())
    }
}

fn factorial_prog<V: Value>(n: i64) -> Vec<Instr<V>> {
    use Instr::*;
    vec![
        Push(V::from_int(n)), Store("n".into()),
        Push(V::from_int(1)), Store("acc".into()),
        // loop at 4: if 0 < n continue else exit
        Push(V::from_int(0)), Load("n".into()), Bin(Op::Lt), Jz(17),
        Load("acc".into()), Load("n".into()), Bin(Op::Mul), Store("acc".into()),
        Load("n".into()), Push(V::from_int(1)), Bin(Op::Sub), Store("n".into()),
        Jmp(4),
        Load("acc".into()), Dup, Print,
    ]
}

fn report<V: Value>(label: &str, prog: &[Instr<V>]) {
    let mut vm: Vm<V> = Vm::new();
    let r = vm.run(prog, 500);
    match r {
        Ok(v) => println!("{}: ok top={:?} out={:?} steps={} vars={}", label, v.map(|x| x.to_string()), vm.out, vm.steps, vm.vars.len()),
        Err(e) => println!("{}: error: {} (out={:?}, steps={})", label, e, vm.out, vm.steps),
    }
}

fn main() {
    report::<i64>("fact10 int", &factorial_prog(10));
    report::<Dyn>("fact5 dyn", &factorial_prog(5));
    report::<i64>("fact25 wrap", &factorial_prog(25));
    use Instr::*;
    report::<Dyn>("strings", &[Push(Dyn::S("ab".into())), Push(Dyn::I(3)), Bin(Op::Mul), Push(Dyn::S("!".into())), Bin(Op::Add), Dup, Print]);
    report::<Dyn>("mixed float", &[Push(Dyn::I(7)), Push(Dyn::F(2.0)), Bin(Op::Div), Dup, Print, Push(Dyn::F(3.5)), Bin(Op::Lt), Print]);
    report::<Dyn>("type err", &[Push(Dyn::B(true)), Push(Dyn::I(1)), Bin(Op::Sub)]);
    report::<Dyn>("eq mixed", &[Push(Dyn::S("x".into())), Push(Dyn::I(1)), Bin(Op::Eq), Print]);
    report::<i64>("div zero", &[Push(1), Push(0), Bin(Op::Div)]);
    report::<i64>("underflow", &[Push(1), Bin(Op::Add)]);
    report::<i64>("unbound", &[Load("zz".into())]);
    report::<i64>("bad jump", &[Push(0), Jz(99)]);
    report::<i64>("infinite", &[Jmp(0)]);
    report::<i64>("halt", &[Push(4), Push(5), Swap, Halt, Print]);
}
