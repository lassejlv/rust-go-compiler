use std::process;
use std::fmt;

#[derive(Debug)]
enum Cmd { Push(i64), Pop, Add, Mul, Dup, Swap, Over, Jz(usize), Jmp(usize), Print, Halt(i32) }

struct Vm { stack: Vec<i64>, pc: usize, steps: usize, out: Vec<String> }

impl fmt::Display for Vm {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "pc={} stack={:?} steps={}", self.pc, self.stack, self.steps) }
}

impl Drop for Vm { fn drop(&mut self) { println!("vm dropped (never printed on exit)"); } }

fn run(vm: &mut Vm, prog: &[Cmd]) -> Result<(), String> {
    loop {
        vm.steps += 1;
        if vm.steps > 200 { return Err(format!("step limit at {}", vm)); }
        let cmd = prog.get(vm.pc).ok_or_else(|| format!("pc out of range: {}", vm.pc))?;
        vm.pc += 1;
        match *cmd {
            Cmd::Push(v) => vm.stack.push(v),
            Cmd::Pop => { vm.stack.pop().ok_or("pop on empty")?; }
            Cmd::Add | Cmd::Mul => {
                let b = vm.stack.pop().ok_or("underflow")?;
                let a = vm.stack.pop().ok_or("underflow")?;
                vm.stack.push(if matches!(cmd, Cmd::Add) { a + b } else { a * b });
            }
            Cmd::Dup => { let t = *vm.stack.last().ok_or("dup empty")?; vm.stack.push(t); }
            Cmd::Over => { let n = vm.stack.len(); if n < 2 { return Err("over".into()); } vm.stack.push(vm.stack[n - 2]); }
            Cmd::Swap => { let n = vm.stack.len(); if n < 2 { return Err("swap".into()); } vm.stack.swap(n - 1, n - 2); }
            Cmd::Jz(t) => { if vm.stack.pop() == Some(0) { vm.pc = t; } }
            Cmd::Jmp(t) => vm.pc = t,
            Cmd::Print => { let s = format!("{:?}", vm.stack.last()); println!("print {}", s); vm.out.push(s); }
            Cmd::Halt(code) => {
                println!("halting with {} after {} steps; {}", code, vm.steps, vm);
                finish(code, &vm.out);
            }
        }
    }
}

fn finish(code: i32, out: &[String]) -> ! {
    println!("outputs: {}", out.len());
    if code != 0 {
        println!("exiting with code {}", code);
    }
    process::exit(code)
}

fn main() {
    // countdown from 5, accumulating product, then halt with product % 256
    let prog = vec![
        Cmd::Push(1), Cmd::Push(5),            // 0,1: acc n
        Cmd::Dup, Cmd::Jz(12),                 // 2,3
        Cmd::Swap, Cmd::Over, Cmd::Mul, Cmd::Swap, // 4-7: acc*n n
        Cmd::Push(-1), Cmd::Add, Cmd::Print,   // 8-10
        Cmd::Jmp(2),                           // 11
        Cmd::Pop, Cmd::Print, Cmd::Halt(3),    // 12-14
    ];
    let mut vm = Vm { stack: vec![], pc: 0, steps: 0, out: vec![] };
    match run(&mut vm, &prog) {
        Ok(()) => println!("finished normally"),
        Err(e) => { println!("error: {}", e); process::exit(2); }
    }
}
