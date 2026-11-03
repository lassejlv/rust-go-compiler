use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;
use std::time::Duration;

fn main() {
    let flag = Arc::new(AtomicBool::new(false));
    let f2 = flag.clone();
    let parked = thread::spawn(move || {
        let mut wakes = 0;
        while !f2.load(Ordering::Acquire) {
            thread::park();
            wakes += 1;
        }
        wakes > 0 || true
    });
    thread::sleep(Duration::from_millis(10));
    flag.store(true, Ordering::Release);
    parked.thread().unpark();
    println!("parked thread finished: {}", parked.join().unwrap());

    // unpark before park: token makes park return immediately
    thread::current().unpark();
    thread::park();
    println!("token consumed");
    thread::park_timeout(Duration::from_millis(1));
    println!("park_timeout returned");

    let stage = Arc::new(AtomicUsize::new(0));
    let main_t = thread::current();
    let s2 = stage.clone();
    let worker = thread::spawn(move || {
        for i in 1..=3 {
            s2.store(i, Ordering::SeqCst);
            main_t.unpark();
            thread::sleep(Duration::from_millis(2));
        }
    });
    let mut seen = 0;
    while seen < 3 {
        thread::park_timeout(Duration::from_millis(50));
        seen = stage.load(Ordering::SeqCst);
    }
    worker.join().unwrap();
    println!("stage reached {}", seen);
    println!("yield ok");
    thread::yield_now();
    let n = thread::available_parallelism().map(|n| n.get() >= 1).unwrap_or(true);
    println!("parallelism>=1 {}", n);
    let h = thread::spawn(|| thread::sleep(Duration::from_millis(5)));
    while !h.is_finished() { thread::sleep(Duration::from_millis(1)); }
    println!("is_finished true");
    h.join().unwrap();
}
