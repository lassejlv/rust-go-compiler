// Panic hook printing to stdout, payload downcasting (&str vs String),
// location line/column, drops during unwinding, catch_unwind around
// arithmetic-overflow/index/unwrap panics, resume_unwind, and a final
// uncaught panic (exit code 101) after output.
use std::panic::{self, AssertUnwindSafe};

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("  drop guard {} (panicking={})", self.0, std::thread::panicking());
    }
}

fn payload_str(p: &(dyn std::any::Any + Send)) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        format!("&str {:?}", s)
    } else if let Some(s) = p.downcast_ref::<String>() {
        format!("String {:?}", s)
    } else if let Some(n) = p.downcast_ref::<i32>() {
        format!("i32 {}", n)
    } else {
        "unknown".to_string()
    }
}

#[inline(never)]
fn add(a: u8, b: u8) -> u8 { a + b }

fn deep(n: u32) -> u32 {
    let _g = Guard(if n == 0 { "deep0" } else { "deepN" });
    if n == 0 {
        let v: Vec<u32> = vec![1, 2, 3];
        let idx = v.len() + 2;
        return v[idx];
    }
    deep(n - 1) + 1
}

fn main() {
    panic::set_hook(Box::new(|info| {
        let loc = info.location().map(|l| format!("line {} col {}", l.line(), l.column())).unwrap_or_default();
        println!("[hook] {} at {}", payload_str(info.payload()), loc);
    }));

    let cases: Vec<(&str, Box<dyn Fn() -> i64>)> = vec![
        ("overflow", Box::new(|| add(200, 100) as i64)),
        ("index", Box::new(|| deep(2) as i64)),
        ("unwrap none", Box::new(|| { let o: Option<i64> = None; o.unwrap() })),
        ("expect err", Box::new(|| "x".parse::<i64>().expect("parse failed"))),
        ("div zero", Box::new(|| { let z = std::hint::black_box(0); 10 / z })),
        ("formatted", Box::new(|| { let n = 7; panic!("bad value {}", n) })),
        ("literal", Box::new(|| panic!("static message"))),
        ("ok", Box::new(|| 99)),
        ("assert_eq", Box::new(|| { assert_eq!(1 + 1, 3, "math is {}", "hard"); 0 })),
        ("shift", Box::new(|| { let s = std::hint::black_box(40u32); (1i32 << s) as i64 })),
        ("neg overflow", Box::new(|| { let m = std::hint::black_box(i32::MIN); (-m) as i64 })),
    ];
    for (name, f) in &cases {
        println!("case {}:", name);
        let r = panic::catch_unwind(AssertUnwindSafe(|| {
            let _g = Guard("case");
            f()
        }));
        match r {
            Ok(v) => println!("  -> ok {}", v),
            Err(p) => println!("  -> caught {}", payload_str(&*p)),
        }
    }

    let r = panic::catch_unwind(|| {
        panic::panic_any(42i32);
    });
    println!("panic_any payload: {}", payload_str(&*r.unwrap_err()));

    let r = panic::catch_unwind(|| {
        let inner = panic::catch_unwind(|| panic!("inner"));
        let p = inner.unwrap_err();
        println!("  re-raising {}", payload_str(&*p));
        panic::resume_unwind(p);
    });
    println!("resume_unwind outer caught: {}", payload_str(&*r.unwrap_err()));

    let _ = panic::take_hook();
    println!("hook removed; final panic next");
    let _g1 = Guard("main-1");
    let v: Vec<i32> = Vec::new();
    let _g2 = Guard("main-2");
    let total: i32 = v.iter().sum::<i32>() + v.first().copied().unwrap_or_else(|| panic!("empty vector"));
    println!("unreachable {}", total);
}
