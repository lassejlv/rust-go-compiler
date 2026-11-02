// Drops during panic unwinding inside catch_unwind: locals, partially built structs,
// vector elements, and nested catch_unwind. Panic hook silenced for deterministic output.
use std::cell::RefCell;
use std::panic::{self, AssertUnwindSafe};

thread_local! {
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}
fn log(s: &str) {
    LOG.with(|l| l.borrow_mut().push(s.to_string()));
}
fn take_log() -> String {
    LOG.with(|l| std::mem::take(&mut *l.borrow_mut()).join(" "))
}

struct Res(&'static str);
impl Drop for Res {
    fn drop(&mut self) {
        log(&format!("~{}", self.0));
    }
}

struct Two {
    _a: Res,
    _b: Res,
}

fn build(fail_at: u32) -> Two {
    let check = |n: u32, name: &'static str| -> Res {
        if n == fail_at {
            panic!("fail building {}", name);
        }
        Res(name)
    };
    Two { _a: check(1, "a"), _b: check(2, "b") }
}

fn deep(n: u32) -> u32 {
    let _r = Res(match n {
        0 => "d0",
        1 => "d1",
        2 => "d2",
        _ => "dn",
    });
    if n == 0 {
        panic!("bottom");
    }
    deep(n - 1) + 1
}

fn msg_of(e: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = e.downcast_ref::<&str>() {
        s.to_string()
    } else if let Some(s) = e.downcast_ref::<String>() {
        s.clone()
    } else {
        "?".to_string()
    }
}

fn main() {
    panic::set_hook(Box::new(|_| {}));

    let r = panic::catch_unwind(|| {
        let _x = Res("x");
        let _y = Res("y");
        panic!("boom");
    });
    println!("1: err={} log=[{}]", r.is_err(), take_log());

    for at in [1, 2, 3] {
        let r = panic::catch_unwind(|| {
            let t = build(at);
            log("built");
            drop(t);
        });
        let m = match &r {
            Ok(()) => "ok".to_string(),
            Err(e) => msg_of(e),
        };
        println!("2.{}: {} log=[{}]", at, m, take_log());
    }

    let r = panic::catch_unwind(|| deep(2));
    println!("3: {} log=[{}]", r.map_err(|e| msg_of(&e)).unwrap_err(), take_log());

    let r = panic::catch_unwind(|| {
        let v = vec![Res("v0"), Res("v1"), Res("v2")];
        let idx = v.len() + 1;
        let _boom = &v[idx];
    });
    println!("4: err={} log=[{}]", r.is_err(), take_log());

    let mut counter = 0;
    let r = panic::catch_unwind(AssertUnwindSafe(|| {
        let _outer = Res("outer");
        let inner = panic::catch_unwind(|| {
            let _i = Res("inner");
            panic!("inner panic");
        });
        counter += 1;
        log(if inner.is_err() { "inner caught" } else { "inner ok" });
        let n: Option<u32> = None;
        counter += n.expect("expect failed") as i32;
    }));
    println!("5: {} counter={} log=[{}]", msg_of(&r.unwrap_err()), counter, take_log());

    let ok = panic::catch_unwind(|| {
        let _a = Res("fine");
        7 * 6
    });
    println!("6: {:?} log=[{}]", ok.ok(), take_log());

    let r = panic::catch_unwind(|| {
        let o: Option<Res> = Some(Res("opt"));
        let v: Result<u8, String> = Err("bad value".into());
        let _keep = o;
        v.unwrap()
    });
    println!("7: {} log=[{}]", msg_of(&r.unwrap_err()), take_log());
}
