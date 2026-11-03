// catch_unwind returning values, panic payload downcasting, resume_unwind, panic hooks, nested catches, drops
use std::panic::{self, AssertUnwindSafe};
use std::cell::RefCell;

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

fn payload_str(p: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = p.downcast_ref::<&str>() {
        format!("&str: {}", s)
    } else if let Some(s) = p.downcast_ref::<String>() {
        format!("String: {}", s)
    } else if let Some(n) = p.downcast_ref::<i32>() {
        format!("i32: {}", n)
    } else {
        "unknown payload".to_string()
    }
}

fn risky(n: i32) -> i32 {
    let _d = D("risky-local");
    match n {
        0 => panic!("static message"),
        1 => panic!("formatted {}", n * 10),
        2 => std::panic::panic_any(42i32),
        3 => {
            let v: Vec<i32> = vec![];
            v[5]
        }
        4 => None::<i32>.unwrap(),
        5 => Err::<i32, &str>("bad").expect("expect failed"),
        6 => {
            let r: Result<i32, String> = Err("e".into());
            r.unwrap()
        }
        _ => n * 2,
    }
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    for n in 0..8 {
        let r = panic::catch_unwind(|| risky(n));
        match r {
            Ok(v) => println!("{} -> ok {}", n, v),
            Err(p) => println!("{} -> panic {}", n, payload_str(&p)),
        }
    }
    let counter = RefCell::new(0);
    let r = panic::catch_unwind(AssertUnwindSafe(|| {
        *counter.borrow_mut() += 1;
        let inner = panic::catch_unwind(|| {
            let _d = D("inner");
            panic!("inner boom");
        });
        println!("inner caught: {}", inner.is_err());
        *counter.borrow_mut() += 10;
        let _d = D("outer");
        if *counter.borrow() > 5 {
            panic!("outer boom");
        }
        7
    }));
    println!("outer: {:?} counter={}", r.map_err(|p| payload_str(&p)), counter.borrow());
    let r2 = panic::catch_unwind(|| {
        let r = panic::catch_unwind(|| panic!("first"));
        if let Err(p) = r {
            println!("rethrowing {}", payload_str(&p));
            panic::resume_unwind(p);
        }
    });
    println!("after resume: {}", r2.map_err(|p| payload_str(&p)).unwrap_err());
    let mut data = vec![1, 2, 3];
    let r3 = panic::catch_unwind(AssertUnwindSafe(|| {
        data.push(4);
        if data.len() > 3 {
            panic!("too long: {}", data.len());
        }
    }));
    println!("{:?} {:?}", r3.is_err(), data);
    let _ = panic::take_hook();
    panic::set_hook(Box::new(|info| {
        let msg = info.payload().downcast_ref::<&str>().copied().unwrap_or("?");
        println!("HOOK: {} at line? {}", msg, info.location().is_some());
    }));
    let _ = panic::catch_unwind(|| panic!("with hook"));
    let _ = panic::take_hook();
    panic::set_hook(Box::new(|_| {}));
    let results: Vec<Result<i32, String>> = (0..4)
        .map(|i| panic::catch_unwind(move || if i % 2 == 1 { panic!("odd {}", i) } else { i }).map_err(|p| payload_str(&p)))
        .collect();
    println!("{:?}", results);
    let h = std::thread::spawn(|| {
        let _d = D("thread-local guard");
        panic!("thread panic");
    });
    let jr = h.join();
    println!("thread joined err={} {}", jr.is_err(), payload_str(&jr.unwrap_err()));
    println!("done");
}
