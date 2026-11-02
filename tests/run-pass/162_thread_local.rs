use std::cell::{Cell, RefCell};
use std::thread;

thread_local! {
    static COUNTER: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

thread_local! {
    static DEPTH: Cell<usize> = const { Cell::new(0) };
}

fn bump() -> u32 {
    COUNTER.with(|c| {
        c.set(c.get() + 1);
        c.get()
    })
}

fn log(msg: &str) {
    LOG.with(|l| l.borrow_mut().push(msg.to_string()));
}

fn recurse(n: u32) -> usize {
    DEPTH.with(|d| d.set(d.get() + 1));
    let max = if n > 0 { recurse(n - 1) } else { DEPTH.with(|d| d.get()) };
    DEPTH.with(|d| d.set(d.get() - 1));
    max
}

fn main() {
    bump();
    bump();
    log("main-1");
    let hs: Vec<_> = (0..3)
        .map(|i| {
            thread::spawn(move || {
                for _ in 0..(i + 1) * 2 {
                    bump();
                }
                log(&format!("t{}", i));
                let count = COUNTER.with(|c| c.get());
                let entries = LOG.with(|l| l.borrow().clone());
                (i, count, entries)
            })
        })
        .collect();
    for h in hs {
        let (i, count, entries) = h.join().unwrap();
        println!("thread {} count {} log {:?}", i, count, entries);
    }
    log("main-2");
    println!("main count {}", COUNTER.with(|c| c.get()));
    LOG.with(|l| println!("main log {:?}", l.borrow()));
    println!("max depth {}", recurse(6));
    println!("depth after {}", DEPTH.with(|d| d.get()));
    COUNTER.set(100);
    println!("set/get {}", COUNTER.get());
    let taken = LOG.take();
    println!("taken {:?} now empty {}", taken, LOG.with_borrow(|l| l.is_empty()));
    LOG.with_borrow_mut(|l| l.push("again".into()));
    println!("{:?}", LOG.with_borrow(|l| l.clone()));
    let child = thread::spawn(|| COUNTER.get()).join().unwrap();
    println!("fresh thread sees {}", child);
}
