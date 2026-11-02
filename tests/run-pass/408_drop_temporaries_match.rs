// Temporary lifetimes: temporaries in match scrutinees, if-let, while-let, let statements,
// function call arguments, and tail expressions of blocks (2021 edition rules).
use std::cell::{Cell, RefCell};

thread_local! {
    static EVENTS: RefCell<Vec<String>> = RefCell::new(Vec::new());
    static NEXT: Cell<u32> = Cell::new(0);
}

fn ev(s: impl Into<String>) {
    EVENTS.with(|e| e.borrow_mut().push(s.into()));
}
fn dump(title: &str) {
    let v = EVENTS.with(|e| std::mem::take(&mut *e.borrow_mut()));
    println!("{}: {}", title, v.join(" | "));
}

struct Guard {
    id: u32,
}
impl Guard {
    fn new() -> Guard {
        let id = NEXT.with(|n| {
            let v = n.get();
            n.set(v + 1);
            v
        });
        ev(format!("new{}", id));
        Guard { id }
    }
    fn value(&self) -> Option<u32> {
        if self.id % 2 == 0 {
            Some(self.id * 10)
        } else {
            None
        }
    }
    fn id(&self) -> u32 {
        self.id
    }
}
impl Drop for Guard {
    fn drop(&mut self) {
        ev(format!("drop{}", self.id));
    }
}

fn takes(a: u32, b: u32) -> u32 {
    ev(format!("takes({},{})", a, b));
    a + b
}

fn main() {
    match Guard::new().value() {
        Some(v) => ev(format!("arm some {}", v)),
        None => ev("arm none"),
    }
    ev("after match");
    dump("match scrutinee");

    if let Some(v) = Guard::new().value() {
        ev(format!("then {}", v));
    } else {
        ev("else");
    }
    ev("after if-let");
    dump("if-let odd");

    if let Some(v) = Guard::new().value() {
        ev(format!("then {}", v));
    }
    ev("after if-let");
    dump("if-let even");

    let x = Guard::new().id();
    ev(format!("let x={}", x));
    dump("let temp");

    let r = takes(Guard::new().id(), Guard::new().id());
    ev(format!("r={}", r));
    dump("call args");

    let mut stack = vec![1, 2, 3];
    let g = RefCell::new(Vec::<u32>::new());
    while let Some(top) = stack.pop() {
        let tmp = Guard::new();
        g.borrow_mut().push(top + tmp.id());
        ev(format!("loop {}", top));
    }
    ev(format!("collected {:?}", g.borrow()));
    dump("while-let");

    let y = {
        let inner = Guard::new();
        ev("block body");
        inner.id() + Guard::new().id()
    };
    ev(format!("y={}", y));
    dump("block tail");

    let held = Guard::new();
    let r2 = &held;
    let v = match r2.value() {
        Some(n) => n,
        None => Guard::new().id() * 100,
    };
    ev(format!("v={}", v));
    drop(held);
    dump("arm temp");

    let cell = RefCell::new(5);
    let doubled = match *cell.borrow() {
        n if n > 3 => n * 2,
        n => n,
    };
    *cell.borrow_mut() += doubled;
    println!("cell = {}", cell.borrow());

    let s = String::from("abc");
    let n = match s.as_str().len() {
        0 => "empty",
        1..=3 => "short",
        _ => "long",
    };
    println!("{} is {}", s, n);
}
