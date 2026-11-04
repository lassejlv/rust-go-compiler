// thread_local! with Cell/RefCell, const init, destructors running at thread exit.
use std::cell::{Cell, RefCell};
use std::sync::mpsc;
use std::sync::Mutex;
use std::thread;

static DROPPED: Mutex<Vec<String>> = Mutex::new(Vec::new());

struct Guard {
    name: String,
    count: Cell<u32>,
}
impl Drop for Guard {
    fn drop(&mut self) {
        DROPPED.lock().unwrap().push(format!("{}:{}", self.name, self.count.get()));
    }
}

thread_local! {
    static COUNTER: Cell<u32> = const { Cell::new(0) };
    static NAMES: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static GUARD: Guard = Guard { name: thread::current().name().unwrap_or("?").to_string(), count: Cell::new(0) };
}

fn bump() -> u32 {
    COUNTER.with(|c| {
        c.set(c.get() + 1);
        c.get()
    })
}

fn main() {
    bump();
    bump();
    let (tx, rx) = mpsc::channel();
    let hs: Vec<_> = (0..3)
        .map(|i| {
            let tx = tx.clone();
            thread::Builder::new()
                .name(format!("t{}", i))
                .spawn(move || {
                    for _ in 0..=i {
                        bump();
                    }
                    NAMES.with_borrow_mut(|v| v.push(format!("n{}", i)));
                    GUARD.with(|g| g.count.set(COUNTER.get() * 10));
                    tx.send((i, COUNTER.get(), NAMES.with_borrow(|v| v.clone()))).unwrap();
                })
                .unwrap()
        })
        .collect();
    drop(tx);
    for h in hs {
        h.join().unwrap();
    }
    let mut res: Vec<_> = rx.iter().collect();
    res.sort();
    println!("{:?}", res);
    println!("main counter {}", COUNTER.get());
    COUNTER.set(100);
    println!("main counter {}", COUNTER.with(|c| c.get()));
    println!("replace -> {}", COUNTER.replace(7));
    println!("take -> {}", COUNTER.take());
    NAMES.set(vec!["x".into()]);
    println!("names {:?}", NAMES.take());
    let mut d = DROPPED.lock().unwrap().clone();
    d.sort();
    println!("dropped {:?}", d);
    // thread that never touches GUARD does not run its destructor
    thread::Builder::new().name("idle".into()).spawn(|| bump()).unwrap().join().unwrap();
    println!("dropped count {}", DROPPED.lock().unwrap().len());
}
