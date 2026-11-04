// thread::park/unpark, is_finished, ThreadId, try_lock, resume_unwind, AssertUnwindSafe,
// #[track_caller] Location, Mutex::get_mut/into_inner, Atomic from_ptr/as_ptr.
use std::panic::{self, AssertUnwindSafe, Location};
use std::sync::atomic::{AtomicBool, AtomicU32, Ordering};
use std::sync::{Arc, Mutex, TryLockError};
use std::thread;
use std::time::Duration;

#[track_caller]
fn where_am_i() -> (u32, u32) {
    let l = Location::caller();
    (l.line(), l.column())
}

#[track_caller]
fn checked_div(a: i32, b: i32) -> i32 {
    if b == 0 {
        let loc = Location::caller();
        panic!("div by zero at line {}", loc.line());
    }
    a / b
}

fn main() {
    let flag = Arc::new(AtomicBool::new(false));
    let f2 = flag.clone();
    let parked = thread::spawn(move || {
        while !f2.load(Ordering::Acquire) {
            thread::park();
        }
        "woken"
    });
    thread::sleep(Duration::from_millis(10));
    flag.store(true, Ordering::Release);
    parked.thread().unpark();
    println!("{}", parked.join().unwrap());

    let h = thread::spawn(|| 5);
    while !h.is_finished() {
        thread::yield_now();
    }
    println!("finished {}", h.join().unwrap());
    let id_main = thread::current().id();
    let id_other = thread::spawn(|| thread::current().id()).join().unwrap();
    println!("ids differ {} same {}", id_main != id_other, id_main == thread::current().id());

    let m = Mutex::new(1);
    {
        let _g = m.lock().unwrap();
        match m.try_lock() {
            Err(TryLockError::WouldBlock) => println!("would block"),
            _ => println!("??"),
        }
    }
    println!("try_lock ok {:?}", m.try_lock().map(|g| *g).ok());
    let mut m = m;
    *m.get_mut().unwrap() += 10;
    println!("into_inner {}", m.into_inner().unwrap());

    panic::set_hook(Box::new(|_| {}));
    let r = panic::catch_unwind(|| {
        let r2 = panic::catch_unwind(|| panic!("inner"));
        if let Err(p) = r2 {
            println!("rethrowing");
            panic::resume_unwind(p);
        }
    });
    println!("outer caught: {:?}", r.unwrap_err().downcast_ref::<&str>());
    let mut v = vec![1, 2];
    let r = panic::catch_unwind(AssertUnwindSafe(|| {
        v.push(3);
        if v.len() > 2 {
            panic!("len {}", v.len());
        }
    }));
    println!("v {:?} err {:?}", v, r.unwrap_err().downcast_ref::<String>());
    let (l1, c1) = where_am_i();
    let (l2, _) = where_am_i();
    println!("lines differ by {} col {}", l2 - l1, c1);
    let r = panic::catch_unwind(|| checked_div(1, 0));
    println!("{:?}", r.unwrap_err().downcast_ref::<String>().map(|s| s.starts_with("div by zero at line")));
    println!("{}", checked_div(9, 3));

    let mut raw = 5u32;
    let a = unsafe { AtomicU32::from_ptr(&mut raw) };
    a.fetch_add(3, Ordering::SeqCst);
    println!("from_ptr {}", raw);
    let at = AtomicU32::new(1);
    unsafe { *at.as_ptr() = 77 };
    println!("as_ptr {}", at.load(Ordering::Relaxed));
    let nb = AtomicBool::new(false);
    println!("fetch_not {} {}", nb.fetch_not(Ordering::SeqCst), nb.load(Ordering::SeqCst));
    println!("parallelism>0 {}", thread::available_parallelism().map(|n| n.get() > 0).unwrap_or(true));
    let h = thread::Builder::new().stack_size(256 * 1024).spawn(|| {
        fn rec(n: u32) -> u32 { if n == 0 { 0 } else { 1 + rec(n - 1) } }
        rec(1000)
    }).unwrap();
    println!("deep {}", h.join().unwrap());
    let r: Result<u32, Box<dyn std::error::Error + Send + Sync>> = thread::spawn(|| {
        let n: u32 = "12".parse()?;
        Ok(n * 2)
    }).join().unwrap();
    println!("{:?}", r.ok());
}
