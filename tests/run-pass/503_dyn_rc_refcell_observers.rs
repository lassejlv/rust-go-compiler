// trait objects behind Rc<RefCell<>>, Weak back-references, observers
use std::cell::RefCell;
use std::rc::{Rc, Weak};

trait Observer {
    fn notify(&mut self, topic: &str, value: i32);
    fn report(&self) -> String;
    fn id(&self) -> u32;
}

struct Counter {
    id: u32,
    count: usize,
}
struct Accum {
    id: u32,
    total: i64,
    history: Vec<i32>,
}
struct Filtered {
    id: u32,
    topic: String,
    hits: Vec<(String, i32)>,
}

impl Observer for Counter {
    fn notify(&mut self, _t: &str, _v: i32) {
        self.count += 1;
    }
    fn report(&self) -> String {
        format!("Counter#{} count={}", self.id, self.count)
    }
    fn id(&self) -> u32 {
        self.id
    }
}
impl Observer for Accum {
    fn notify(&mut self, _t: &str, v: i32) {
        self.total += v as i64;
        self.history.push(v);
        if self.history.len() > 3 {
            self.history.remove(0);
        }
    }
    fn report(&self) -> String {
        format!("Accum#{} total={} last={:?}", self.id, self.total, self.history)
    }
    fn id(&self) -> u32 {
        self.id
    }
}
impl Observer for Filtered {
    fn notify(&mut self, t: &str, v: i32) {
        if t == self.topic {
            self.hits.push((t.to_string(), v));
        }
    }
    fn report(&self) -> String {
        format!("Filtered#{}({}) hits={:?}", self.id, self.topic, self.hits)
    }
    fn id(&self) -> u32 {
        self.id
    }
}

struct Subject {
    observers: Vec<Weak<RefCell<dyn Observer>>>,
}

impl Subject {
    fn attach(&mut self, o: &Rc<RefCell<dyn Observer>>) {
        self.observers.push(Rc::downgrade(o));
    }
    fn publish(&mut self, topic: &str, v: i32) -> usize {
        let mut delivered = 0;
        self.observers.retain(|w| match w.upgrade() {
            Some(o) => {
                o.borrow_mut().notify(topic, v);
                delivered += 1;
                true
            }
            None => false,
        });
        delivered
    }
}

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

fn path(n: &Rc<Node>) -> String {
    let mut parts = vec![n.name.clone()];
    let mut cur = n.parent.borrow().upgrade();
    while let Some(p) = cur {
        parts.push(p.name.clone());
        cur = p.parent.borrow().upgrade();
    }
    parts.reverse();
    parts.join("/")
}

fn main() {
    let c: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(Counter { id: 1, count: 0 }));
    let a: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(Accum { id: 2, total: 0, history: vec![] }));
    let mut subj = Subject { observers: vec![] };
    subj.attach(&c);
    subj.attach(&a);
    {
        let f: Rc<RefCell<dyn Observer>> = Rc::new(RefCell::new(Filtered { id: 3, topic: "temp".into(), hits: vec![] }));
        subj.attach(&f);
        println!("delivered {}", subj.publish("temp", 20));
        println!("delivered {}", subj.publish("hum", 55));
        println!("{}", f.borrow().report());
        println!("strong={} weak={}", Rc::strong_count(&f), Rc::weak_count(&f));
    }
    println!("delivered {}", subj.publish("temp", 21));
    for v in [1, -2, 3, -4] {
        subj.publish("x", v);
    }
    println!("observers alive: {}", subj.observers.len());
    let all = vec![c.clone(), a.clone()];
    for o in &all {
        let b = o.borrow();
        println!("{} -> {}", b.id(), b.report());
    }
    let shared = Rc::clone(&a);
    shared.borrow_mut().notify("direct", 100);
    println!("{}", a.borrow().report());
    println!("rc counts: {} {}", Rc::strong_count(&a), Rc::strong_count(&c));

    let root = Rc::new(Node { name: "root".into(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    let usr = Rc::new(Node { name: "usr".into(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    let bin = Rc::new(Node { name: "bin".into(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    *usr.parent.borrow_mut() = Rc::downgrade(&root);
    root.children.borrow_mut().push(usr.clone());
    *bin.parent.borrow_mut() = Rc::downgrade(&usr);
    usr.children.borrow_mut().push(bin.clone());
    println!("{}", path(&bin));
    println!("children of root: {}", root.children.borrow().len());
    println!("usr strong={} weak={}", Rc::strong_count(&usr), Rc::weak_count(&usr));
    drop(root);
    println!("after drop: parent of usr alive = {}", usr.parent.borrow().upgrade().is_some());
    println!("{}", path(&bin));
}
