// assert! messages, panic payloads captured with catch_unwind + custom hook
use std::panic;

fn msg(r: std::thread::Result<()>) -> String {
    match r {
        Ok(()) => "ok".into(),
        Err(e) => {
            if let Some(s) = e.downcast_ref::<&str>() { format!("str: {}", s) }
            else if let Some(s) = e.downcast_ref::<String>() { format!("String: {}", s) }
            else { "other".into() }
        }
    }
}

macro_rules! check {
    ($cond:expr) => { assert!($cond) };
    ($cond:expr, $($arg:tt)+) => { assert!($cond, $($arg)+) };
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    let x = 3;
    println!("{}", msg(panic::catch_unwind(|| assert!(x == 4))));
    println!("{}", msg(panic::catch_unwind(|| assert!(x == 4, "x was {}", x))));
    println!("{}", msg(panic::catch_unwind(|| assert!(x == 4, "x was {x}"))));
    println!("{}", msg(panic::catch_unwind(|| assert!(x == 4, "static msg"))));
    println!("{}", msg(panic::catch_unwind(|| assert_eq!(x, 4))));
    println!("{}", msg(panic::catch_unwind(|| assert_eq!(x + 1, 5, "fine"))));
    println!("{}", msg(panic::catch_unwind(|| assert_eq!(vec![1, 2], vec![1], "vecs {:?}", (1, 2)))));
    println!("{}", msg(panic::catch_unwind(|| assert_ne!("a", "a"))));
    println!("{}", msg(panic::catch_unwind(|| check!(x > 10))));
    println!("{}", msg(panic::catch_unwind(|| check!(x > 10, "too small: {}", x))));
    println!("{}", msg(panic::catch_unwind(|| check!(x < 10))));
    println!("{}", msg(panic::catch_unwind(|| panic!())));
    println!("{}", msg(panic::catch_unwind(|| panic!("plain"))));
    println!("{}", msg(panic::catch_unwind(|| panic!("fmt {} {x}", 1))));
    println!("{}", msg(panic::catch_unwind(|| { let v: Vec<i32> = vec![]; let _ = v[3]; })));
    println!("{}", msg(panic::catch_unwind(|| { let o: Option<u8> = None; o.unwrap(); })));
    println!("{}", msg(panic::catch_unwind(|| { let o: Option<u8> = None; o.expect("needed"); })));
    println!("{}", msg(panic::catch_unwind(|| { let r: Result<u8, String> = Err("bad".into()); r.unwrap(); })));
    println!("{}", msg(panic::catch_unwind(|| unreachable!())));
    println!("{}", msg(panic::catch_unwind(|| unreachable!("state {}", 7))));
    println!("{}", msg(panic::catch_unwind(|| todo!())));
    println!("{}", msg(panic::catch_unwind(|| unimplemented!("feature {}", "x"))));
    println!("{}", msg(panic::catch_unwind(|| panic::panic_any(5u8))));
    println!("{}", msg(panic::catch_unwind(|| debug_assert_eq!(1, 2))));
    println!("{}", msg(panic::catch_unwind(|| assert!(matches!(Some(1), None), "{:?}", Some(1)))));
    println!("{}", msg(panic::catch_unwind(|| { let a = [1u8]; let i = 5; let _ = a.get(i).copied().unwrap_or_else(|| panic!("idx {i}")); })));
}
