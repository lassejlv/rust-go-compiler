// std::process::exit with a non-zero code after printing; destructors do not run, stdout must be flushed
use std::io::Write;

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

#[derive(Debug)]
enum Status {
    Ok,
    Warn(u32),
    Fail(String),
}

fn check(items: &[i32]) -> Status {
    let neg = items.iter().filter(|x| **x < 0).count() as u32;
    if items.iter().any(|x| *x > 1000) {
        Status::Fail(format!("value too large in {:?}", items))
    } else if neg > 0 {
        Status::Warn(neg)
    } else {
        Status::Ok
    }
}

fn code_for(s: &Status) -> i32 {
    match s {
        Status::Ok => 0,
        Status::Warn(n) => *n as i32,
        Status::Fail(_) => 3,
    }
}

fn main() {
    let _guard = Noisy("guard");
    {
        let _inner = Noisy("inner");
        println!("in scope");
    }
    let batches = vec![vec![1, 2, 3], vec![-1, 5, -7], vec![10, 2000]];
    let mut worst = 0;
    for (i, b) in batches.iter().enumerate() {
        let st = check(b);
        let c = code_for(&st);
        println!("batch {} -> {:?} (code {})", i, st, c);
        worst = worst.max(c);
    }
    let summary: Vec<String> = batches
        .iter()
        .map(|b| format!("{}:{}", b.len(), b.iter().sum::<i32>()))
        .collect();
    println!("summary {}", summary.join(","));
    fn helper(depth: u32) -> u32 {
        let _n = Noisy("helper");
        if depth == 0 { 0 } else { depth + helper(depth - 1) }
    }
    println!("helper {}", helper(2));
    let status_list = [Status::Ok, Status::Warn(1), Status::Fail("x".into())];
    let codes: Vec<i32> = status_list.iter().map(code_for).collect();
    println!("codes {:?}", codes);
    print!("partial line without newline; ");
    let out = std::io::stdout();
    let mut lock = out.lock();
    writeln!(lock, "worst = {}", worst).unwrap();
    drop(lock);
    if worst >= 3 {
        print!("exiting with {}", worst);
        std::process::exit(worst);
    }
    println!("not reached");
}
