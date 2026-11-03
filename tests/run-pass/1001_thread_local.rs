use std::cell::{Cell, RefCell};
use std::thread;

thread_local! {
    static COUNTER: Cell<u32> = Cell::new(0);
    static LOG: RefCell<Vec<String>> = RefCell::new(Vec::new());
}

thread_local!(static CONST_TL: u64 = const { 99 });

struct Guard(&'static str);
impl Drop for Guard {
    fn drop(&mut self) {
        println!("tls dtor {}", self.0);
    }
}
thread_local!(static G: RefCell<Option<Guard>> = RefCell::new(None));

fn bump() -> u32 {
    COUNTER.with(|c| {
        c.set(c.get() + 1);
        c.get()
    })
}

fn main() {
    bump();
    bump();
    println!("main counter {}", bump());
    LOG.with(|l| l.borrow_mut().push("main".into()));
    let h = thread::spawn(|| {
        let before = COUNTER.with(|c| c.get());
        for _ in 0..10 { bump(); }
        LOG.with(|l| l.borrow_mut().push("child".into()));
        let log = LOG.with(|l| l.borrow().clone());
        G.with(|g| *g.borrow_mut() = Some(Guard("child-guard")));
        (before, COUNTER.with(|c| c.get()), log)
    });
    let r = h.join().unwrap();
    println!("child {:?}", r);
    println!("main still {}", COUNTER.with(Cell::get));
    LOG.with(|l| println!("main log {:?}", l.borrow()));
    println!("const tl {}", CONST_TL.with(|v| *v));
    COUNTER.set(50);
    println!("set/get {}", COUNTER.get());
    LOG.with_borrow_mut(|v| v.push("again".into()));
    println!("with_borrow len {}", LOG.with_borrow(|v| v.len()));
    let old = COUNTER.replace(7);
    println!("replace old {} new {}", old, COUNTER.take());
    let hs: Vec<_> = (0..3).map(|i| thread::spawn(move || {
        for _ in 0..i { bump(); }
        COUNTER.get()
    })).collect();
    let mut v: Vec<u32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    v.sort();
    println!("{:?}", v);
    println!("try_with ok {}", COUNTER.try_with(|c| c.get()).is_ok());
}
