// std::process::exit from deep inside nested calls and closures: pending
// print! output (no newline) must be flushed, destructors must NOT run, and
// the exit code is propagated.
use std::collections::BTreeMap;
use std::io::Write;

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) { println!("drop {}", self.0); }
}

#[derive(Debug)]
enum Cmd { Push(i64), Add, Mul, Dup, Print, Exit }

fn parse(src: &str) -> Vec<Cmd> {
    src.split_whitespace()
        .map(|t| match t {
            "+" => Cmd::Add,
            "*" => Cmd::Mul,
            "dup" => Cmd::Dup,
            "." => Cmd::Print,
            "exit" => Cmd::Exit,
            n => Cmd::Push(n.parse().unwrap()),
        })
        .collect()
}

fn run(prog: &[Cmd], stats: &mut BTreeMap<&'static str, u32>) {
    let _n = Noisy("run-frame");
    let mut stack: Vec<i64> = Vec::new();
    for c in prog {
        let key = match c { Cmd::Push(_) => "push", Cmd::Add => "add", Cmd::Mul => "mul", Cmd::Dup => "dup", Cmd::Print => "print", Cmd::Exit => "exit" };
        *stats.entry(key).or_insert(0) += 1;
        match c {
            Cmd::Push(n) => stack.push(*n),
            Cmd::Add => { let b = stack.pop().unwrap(); let a = stack.pop().unwrap(); stack.push(a + b) }
            Cmd::Mul => { let b = stack.pop().unwrap(); let a = stack.pop().unwrap(); stack.push(a * b) }
            Cmd::Dup => { let t = *stack.last().unwrap(); stack.push(t) }
            Cmd::Print => print!("{} ", stack.last().unwrap()),
            Cmd::Exit => {
                let code = stack.pop().unwrap_or(0) as i32;
                print!("| stats {:?} | exiting with {}", stats, code);
                let finish = |c: i32| -> ! {
                    let _inner = Noisy("closure");
                    std::process::exit(c)
                };
                finish(code);
            }
        }
    }
    println!();
}

fn main() {
    let _top = Noisy("main");
    let mut stats = BTreeMap::new();
    run(&parse("1 2 + . dup * . 10 *"), &mut stats);
    println!("after first run: {:?}", stats);
    {
        let _scoped = Noisy("scoped");
    }
    let mut out = std::io::stdout().lock();
    write!(out, "locked write without newline; ").unwrap();
    drop(out);
    let v = vec![Noisy("vec-elem")];
    run(&parse("3 4 * . 5 + . 7 exit 100 ."), &mut stats);
    println!("never printed {}", v.len());
}
