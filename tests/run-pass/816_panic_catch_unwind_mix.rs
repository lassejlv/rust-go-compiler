use std::panic::{self, AssertUnwindSafe};
use std::cell::RefCell;

struct Guard(&'static str, &'static RefCellLog);
struct RefCellLog;
thread_local! { static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new()); }
static L: RefCellLog = RefCellLog;
impl Drop for Guard { fn drop(&mut self) { LOG.with(|l| l.borrow_mut().push(format!("drop {}", self.0))); } }

fn risky(n: i32) -> i32 {
    let _g = Guard("risky", &L);
    if n < 0 { panic!("negative input: {}", n); }
    if n == 0 { let v: Vec<i32> = vec![]; return v[n as usize]; }
    if n == 13 { std::panic::panic_any(13u32); }
    if n == 7 { None::<i32>.expect("seven is unlucky"); }
    if n == 8 { let r: Result<i32, String> = Err("eight".into()); return r.unwrap(); }
    100 / n
}

fn describe(payload: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = payload.downcast_ref::<&str>() { format!("&str: {}", s) }
    else if let Some(s) = payload.downcast_ref::<String>() { format!("String: {}", s) }
    else if let Some(n) = payload.downcast_ref::<u32>() { format!("u32: {}", n) }
    else { "unknown".to_string() }
}

fn main() {
    panic::set_hook(Box::new(|_| {}));
    for n in [5, -3, 0, 13, 7, 8, 20] {
        let r = panic::catch_unwind(|| risky(n));
        match r {
            Ok(v) => println!("risky({}) = {}", n, v),
            Err(p) => println!("risky({}) panicked with {}", n, describe(&p)),
        }
    }
    let mut counter = 0;
    let r = panic::catch_unwind(AssertUnwindSafe(|| {
        counter += 1;
        let _a = Guard("a", &L);
        let _b = Guard("b", &L);
        if counter == 1 { panic!("boom"); }
        counter += 100;
    }));
    println!("counter={} is_err={}", counter, r.is_err());
    let overflow = panic::catch_unwind(|| { let x: u8 = std::hint::black_box(250); x + 10 });
    println!("overflow caught: {:?}", overflow.map_err(|p| describe(&p)));
    let divz = panic::catch_unwind(|| { let z = std::hint::black_box(0); 10 / z });
    println!("div caught: {:?}", divz.map_err(|p| describe(&p)));
    let idx = panic::catch_unwind(|| { let v = vec![1, 2, 3]; let i = std::hint::black_box(10); v[i] });
    println!("index caught: {:?}", idx.map_err(|p| describe(&p)));
    let slice = panic::catch_unwind(|| { let s = "héllo"; let i = std::hint::black_box(2); s[..i].len() });
    println!("slice caught: {:?}", slice.map_err(|p| describe(&p)));
    let refcell = panic::catch_unwind(|| { let c = RefCell::new(1); let _b = c.borrow_mut(); let _c = c.borrow_mut(); });
    println!("refcell caught: {:?}", refcell.map_err(|p| describe(&p)));
    let nested = panic::catch_unwind(|| {
        let inner = panic::catch_unwind(|| panic!("inner"));
        println!("inner was err: {}", inner.is_err());
        panic!("outer {}", 2);
    });
    println!("nested: {:?}", nested.map_err(|p| describe(&p)));
    let assert_r = panic::catch_unwind(|| { assert_eq!(1 + 1, 3, "math is {}", "broken"); });
    println!("assert: {:?}", assert_r.map_err(|p| describe(&p)));
    let assert_r2 = panic::catch_unwind(|| { assert!(false); });
    println!("assert2: {:?}", assert_r2.map_err(|p| describe(&p)));
    let un = panic::catch_unwind(|| -> i32 { unimplemented!("later") });
    println!("unimpl: {:?}", un.map_err(|p| describe(&p)));
    let td = panic::catch_unwind(|| -> i32 { todo!() });
    println!("todo: {:?}", td.map_err(|p| describe(&p)));
    let ur = panic::catch_unwind(|| -> i32 { unreachable!("state {}", 3) });
    println!("unreachable: {:?}", ur.map_err(|p| describe(&p)));
    let resumed = panic::catch_unwind(|| {
        let r = panic::catch_unwind(|| panic!("first"));
        if let Err(p) = r { panic::resume_unwind(p); }
    });
    println!("resumed: {:?}", resumed.map_err(|p| describe(&p)));
    LOG.with(|l| for (i, e) in l.borrow().iter().enumerate() { println!("log[{}] {}", i, e); });
    let _ = panic::take_hook();
    let v: Vec<i32> = Vec::new();
    println!("about to fail for real");
    let _ = v.iter().max().unwrap();
}
