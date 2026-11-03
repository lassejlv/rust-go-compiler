// Cell / RefCell interior mutability through shared references.
use std::cell::{Cell, RefCell};

struct Counter {
    hits: Cell<u32>,
    log: RefCell<Vec<String>>,
}

impl Counter {
    fn hit(&self, who: &str) {
        self.hits.set(self.hits.get() + 1);
        self.log.borrow_mut().push(who.to_string());
    }
}

fn main() {
    let c = Counter { hits: Cell::new(0), log: RefCell::new(Vec::new()) };
    let r1 = &c;
    let r2 = &c;
    r1.hit("a");
    r2.hit("b");
    r1.hit("c");
    let n = c.log.borrow().len();
    if let Ok(mut g) = c.log.try_borrow_mut() {
        g.push(format!("n={}", n));
    }
    {
        let g = c.log.borrow();
        let busy = c.log.try_borrow_mut().is_err();
        println!("{} {}", g.len(), busy);
    }
    println!("{} {:?}", c.hits.get(), c.log.borrow());
}
