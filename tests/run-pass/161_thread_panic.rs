use std::sync::{Arc, Mutex};
use std::thread;

fn risky(n: i32) -> i32 {
    if n == 3 {
        panic!("bad input {}", n);
    }
    n * 10
}

fn main() {
    let h = thread::spawn(|| {
        panic!("boom");
    });
    let r: Result<(), _> = h.join();
    match r {
        Ok(_) => println!("thread finished"),
        Err(_) => println!("thread panicked"),
    }

    let handles: Vec<_> = (1..=5).map(|i| thread::spawn(move || risky(i))).collect();
    for (i, h) in handles.into_iter().enumerate() {
        match h.join() {
            Ok(v) => println!("task {} -> {}", i + 1, v),
            Err(e) => {
                let msg = if let Some(s) = e.downcast_ref::<String>() {
                    s.clone()
                } else if let Some(s) = e.downcast_ref::<&str>() {
                    s.to_string()
                } else {
                    "?".to_string()
                };
                println!("task {} panicked: {}", i + 1, msg);
            }
        }
    }

    let h = thread::spawn(|| {
        let v: Vec<i32> = Vec::new();
        v[5]
    });
    println!("index panic caught: {}", h.join().is_err());

    let h = thread::spawn(|| {
        let x: Option<u8> = None;
        x.expect("needed a value")
    });
    if let Err(e) = h.join() {
        let msg = e.downcast_ref::<String>().map(|s| s.as_str()).or(e.downcast_ref::<&str>().copied());
        println!("expect message: {:?}", msg);
    }

    // poisoned mutex
    let m = Arc::new(Mutex::new(0));
    let m2 = Arc::clone(&m);
    let _ = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        *g += 1;
        panic!("while holding lock");
    })
    .join();
    println!("poisoned: {}", m.is_poisoned());
    let v = match m.lock() {
        Ok(g) => *g,
        Err(poison) => *poison.into_inner(),
    };
    println!("recovered value {}", v);

    let r = std::panic::catch_unwind(|| risky(3));
    println!("catch_unwind is_err: {}", r.is_err());
    let r = std::panic::catch_unwind(|| risky(4));
    println!("catch_unwind ok: {:?}", r.ok());
    println!("main continues");
}
