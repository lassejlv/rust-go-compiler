use std::thread;
use std::panic;

#[derive(Debug)]
struct MyErr { code: i32 }

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let h = thread::spawn(|| {
        panic!("boom {}", 42);
    });
    match h.join() {
        Ok(_) => println!("ok?"),
        Err(e) => {
            if let Some(s) = e.downcast_ref::<String>() {
                println!("String payload: {}", s);
            } else if let Some(s) = e.downcast_ref::<&str>() {
                println!("str payload: {}", s);
            }
        }
    }
    let h = thread::spawn(|| -> i32 { panic!("static msg") });
    let e = h.join().unwrap_err();
    println!("is &str: {}", e.downcast_ref::<&'static str>().is_some());
    println!("msg: {:?}", e.downcast_ref::<&'static str>());
    let h = thread::spawn(|| { panic::panic_any(MyErr { code: 7 }); });
    let e = h.join().unwrap_err();
    match e.downcast::<MyErr>() {
        Ok(b) => println!("custom payload {:?}", b),
        Err(_) => println!("unknown"),
    }
    let h = thread::spawn(|| {
        let v: Vec<i32> = vec![];
        v[3]
    });
    let e = h.join().unwrap_err();
    println!("index panic msg: {:?}", e.downcast_ref::<String>());
    let h = thread::spawn(|| { let x: Option<u8> = None; x.unwrap() });
    let e = h.join().unwrap_err();
    println!("unwrap none: {:?}", e.downcast_ref::<&str>().copied().or(e.downcast_ref::<String>().map(|s| s.as_str())));
    let hs: Vec<_> = (0..6).map(|i| thread::spawn(move || {
        if i % 2 == 1 { panic!("odd {}", i); }
        i * 10
    })).collect();
    let rs: Vec<String> = hs.into_iter().map(|h| match h.join() {
        Ok(v) => format!("ok {}", v),
        Err(e) => format!("err {}", e.downcast_ref::<String>().unwrap()),
    }).collect();
    println!("{:?}", rs);
    let r = panic::catch_unwind(|| {
        let h = thread::spawn(|| panic!("inner"));
        h.join().is_err()
    });
    println!("catch_unwind {:?}", r);
    let r = thread::scope(|s| {
        let h = s.spawn(|| panic!("scoped"));
        h.join().is_err()
    });
    println!("scoped join err {}", r);
    let _ = panic::take_hook();
    println!("done");
}
