// Panics in threads: join returns Err with payload downcast; Mutex/RwLock poisoning.
use std::panic;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let h = thread::spawn(|| -> u32 { panic!("static message") });
    match h.join() {
        Ok(v) => println!("ok {}", v),
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&'static str>() {
                println!("str payload: {}", s);
            } else {
                println!("other payload");
            }
        }
    }
    let h = thread::spawn(|| {
        let n = 42;
        panic!("formatted {}", n);
    });
    let e = h.join().unwrap_err();
    println!("is String: {} -> {:?}", e.is::<String>(), e.downcast_ref::<String>());
    let h = thread::spawn(|| std::panic::panic_any(7u64));
    let e = h.join().unwrap_err();
    println!("u64 payload {:?}", e.downcast::<u64>().ok().map(|b| *b));

    // Mutex poisoning
    let m = Arc::new(Mutex::new(vec![1, 2, 3]));
    let m2 = m.clone();
    let r = thread::spawn(move || {
        let mut g = m2.lock().unwrap();
        g.push(4);
        panic!("while holding lock");
    })
    .join();
    println!("thread failed: {}", r.is_err());
    println!("poisoned: {}", m.is_poisoned());
    match m.lock() {
        Ok(_) => println!("not poisoned?"),
        Err(p) => {
            let g = p.into_inner();
            println!("recovered data {:?}", *g);
        }
    }
    m.clear_poison();
    println!("after clear: {} {:?}", m.is_poisoned(), m.lock().map(|g| g.len()));

    // RwLock poisoning via writer panic
    let rw = Arc::new(RwLock::new(String::from("cfg")));
    let rw2 = rw.clone();
    let _ = thread::spawn(move || {
        let mut w = rw2.write().unwrap();
        w.push_str("-partial");
        panic!("writer died");
    })
    .join();
    println!("rw poisoned {}", rw.is_poisoned());
    let s = rw.read().unwrap_or_else(|e| e.into_inner()).clone();
    println!("rw value {}", s);
    // reader panic does not poison
    let rw3 = Arc::new(RwLock::new(1));
    let rw4 = rw3.clone();
    let _ = thread::spawn(move || {
        let _r = rw4.read().unwrap();
        panic!("reader died");
    })
    .join();
    println!("rw3 poisoned {}", rw3.is_poisoned());

    // catch_unwind in the same thread
    let res = panic::catch_unwind(|| {
        let v: Vec<i32> = vec![];
        v[3]
    });
    println!("catch_unwind err {}", res.is_err());
    let ok = panic::catch_unwind(|| 5).unwrap();
    println!("ok {}", ok);
    // scoped thread panic propagates through scope -> catch it
    let res = panic::catch_unwind(|| {
        thread::scope(|s| {
            s.spawn(|| panic!("scoped boom"));
        });
    });
    println!("scope panic caught {}", res.is_err());
    let _ = panic::take_hook();
    println!("done");
}
