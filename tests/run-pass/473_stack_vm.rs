// A stack-based VM with an instruction enum: arithmetic, comparisons, jumps, calls/returns with a
// frame stack, locals, a tiny assembler with labels (two-pass), traps returned as Result, a step
// limit, and disassembly via Display. Programs: factorial, gcd, primes sieve, and error cases.
use std::collections::HashMap;
use std::fmt;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Ins {
    Push(i64), Pop, Dup, Swap, Over, Add, Sub, Mul, Div, Mod, Lt, Eq, Not,
    Jmp(usize), Jz(usize), Call(usize, u8), Ret, Load(u8), Store(u8), Print, Halt,
}

impl fmt::Display for Ins {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            Ins::Push(v) => write!(f, "push {}", v),
            Ins::Jmp(t) | Ins::Jz(t) => write!(f, "{} @{}", if matches!(self, Ins::Jmp(_)) { "jmp" } else { "jz" }, t),
            Ins::Call(t, n) => write!(f, "call @{} /{}", t, n),
            Ins::Load(i) => write!(f, "load {}", i),
            Ins::Store(i) => write!(f, "store {}", i),
            other => write!(f, "{}", format!("{:?}", other).to_lowercase()),
        }
    }
}

#[derive(Debug, PartialEq)]
enum Trap {
    StackUnderflow(usize),
    DivZero(usize),
    BadLocal(usize, u8),
    StepLimit,
    PcOut(usize),
}

struct Frame {
    ret: usize,
    locals: Vec<i64>,
}

fn run(code: &[Ins], limit: usize) -> Result<(Vec<i64>, Vec<i64>, usize), Trap> {
    let mut st: Vec<i64> = vec![];
    let mut out = vec![];
    let mut frames = vec![Frame { ret: usize::MAX, locals: vec![0; 4] }];
    let mut pc = 0;
    let mut steps = 0;
    macro_rules! pop {
        () => {
            st.pop().ok_or(Trap::StackUnderflow(pc))?
        };
    }
    loop {
        steps += 1;
        if steps > limit {
            return Err(Trap::StepLimit);
        }
        let ins = *code.get(pc).ok_or(Trap::PcOut(pc))?;
        pc += 1;
        match ins {
            Ins::Push(v) => st.push(v),
            Ins::Pop => { pop!(); }
            Ins::Dup => { let a = pop!(); st.push(a); st.push(a); }
            Ins::Swap => { let b = pop!(); let a = pop!(); st.push(b); st.push(a); }
            Ins::Over => { let b = pop!(); let a = pop!(); st.extend([a, b, a]); }
            Ins::Add | Ins::Sub | Ins::Mul | Ins::Div | Ins::Mod | Ins::Lt | Ins::Eq => {
                let b = pop!();
                let a = pop!();
                st.push(match ins {
                    Ins::Add => a.wrapping_add(b),
                    Ins::Sub => a.wrapping_sub(b),
                    Ins::Mul => a.wrapping_mul(b),
                    Ins::Div | Ins::Mod if b == 0 => return Err(Trap::DivZero(pc - 1)),
                    Ins::Div => a / b,
                    Ins::Mod => a % b,
                    Ins::Lt => (a < b) as i64,
                    _ => (a == b) as i64,
                });
            }
            Ins::Not => { let a = pop!(); st.push((a == 0) as i64); }
            Ins::Jmp(t) => pc = t,
            Ins::Jz(t) => if pop!() == 0 { pc = t },
            Ins::Call(t, n) => {
                let mut locals = vec![0; 4];
                for i in (0..n as usize).rev() {
                    locals[i] = pop!();
                }
                frames.push(Frame { ret: pc, locals });
                pc = t;
            }
            Ins::Ret => {
                let f = frames.pop().unwrap();
                if f.ret == usize::MAX { return Ok((st, out, steps)); }
                pc = f.ret;
            }
            Ins::Load(i) => { let v = *frames.last().unwrap().locals.get(i as usize).ok_or(Trap::BadLocal(pc - 1, i))?; st.push(v); }
            Ins::Store(i) => { let v = pop!(); *frames.last_mut().unwrap().locals.get_mut(i as usize).ok_or(Trap::BadLocal(pc - 1, i))? = v; }
            Ins::Print => { let v = pop!(); out.push(v); }
            Ins::Halt => return Ok((st, out, steps)),
        }
    }
}

fn assemble(src: &str) -> Vec<Ins> {
    let lines: Vec<&str> = src.lines().flat_map(|l| l.split(';').next().unwrap().split('|')).map(str::trim).filter(|l| !l.is_empty()).collect();
    let mut labels = HashMap::new();
    let mut n = 0;
    for l in &lines {
        if let Some(name) = l.strip_suffix(':') { labels.insert(name, n); } else { n += 1; }
    }
    let target = |s: &str| -> usize { labels[s] };
    lines.iter().filter(|l| !l.ends_with(':')).map(|l| {
        let mut p = l.split_whitespace();
        let op = p.next().unwrap();
        let arg = p.next();
        match (op, arg) {
            ("push", Some(v)) => Ins::Push(v.parse().unwrap()),
            ("jmp", Some(t)) => Ins::Jmp(target(t)),
            ("jz", Some(t)) => Ins::Jz(target(t)),
            ("call", Some(t)) => Ins::Call(target(t), p.next().unwrap().parse().unwrap()),
            ("load", Some(i)) => Ins::Load(i.parse().unwrap()),
            ("store", Some(i)) => Ins::Store(i.parse().unwrap()),
            ("pop", _) => Ins::Pop, ("dup", _) => Ins::Dup, ("swap", _) => Ins::Swap, ("over", _) => Ins::Over,
            ("add", _) => Ins::Add, ("sub", _) => Ins::Sub, ("mul", _) => Ins::Mul, ("div", _) => Ins::Div,
            ("mod", _) => Ins::Mod, ("lt", _) => Ins::Lt, ("eq", _) => Ins::Eq, ("not", _) => Ins::Not,
            ("ret", _) => Ins::Ret, ("print", _) => Ins::Print, ("halt", _) => Ins::Halt,
            other => panic!("bad instruction {:?}", other),
        }
    }).collect()
}

const FACT: &str = "
    push 10 | call fact 1 | print | halt
fact:            ; fact(n) recursive
    load 0 | push 2 | lt | jz recurse
    push 1 | ret
recurse:
    load 0 | load 0 | push 1 | sub | call fact 1 | mul | ret
";

const GCD: &str = "
    push 1071 | push 462
gcd_loop:
    dup | jz done
    swap | over | mod | jmp gcd_loop
done:
    pop | print | halt
";

const PRIMES: &str = "
    push 2 | store 0               ; n = 2
outer:
    load 0 | push 30 | lt | jz end
    push 2 | store 1               ; d = 2
inner:
    load 1 | load 1 | mul | load 0 | swap | lt | not | jz isprime   ; n < d*d ?
    load 0 | load 1 | mod | jz next                                 ; divisible
    load 1 | push 1 | add | store 1 | jmp inner
isprime:
    load 0 | print
next:
    load 0 | push 1 | add | store 0 | jmp outer
end:
    halt
";

fn main() {
    for (name, src) in [("fact", FACT), ("gcd", GCD), ("primes", PRIMES)] {
        let code = assemble(src);
        println!("{} ({} ins): {:?}", name, code.len(), run(&code, 10_000).map(|(s, o, n)| (s, o, n)));
    }
    let code = assemble(GCD);
    for (i, ins) in code.iter().enumerate().take(6) {
        println!("{:>3}: {}", i, ins);
    }
    println!("{:?}", run(&[Ins::Push(1), Ins::Push(0), Ins::Div], 10));
    println!("{:?}", run(&[Ins::Add], 10));
    println!("{:?}", run(&[Ins::Jmp(0)], 50));
    println!("{:?}", run(&[Ins::Push(1), Ins::Store(9)], 10));
    println!("{:?}", run(&[Ins::Push(1)], 10));
    println!("{:?}", run(&[Ins::Push(i64::MAX), Ins::Push(1), Ins::Add, Ins::Print, Ins::Ret], 10));
}
