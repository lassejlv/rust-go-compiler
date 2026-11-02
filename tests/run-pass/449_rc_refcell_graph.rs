// Rc<RefCell<>> graph with shared mutable nodes: adjacency via Rc, cycle detection with Weak
// back-edges, borrow/borrow_mut patterns, try_borrow failures, RefCell::replace/take/swap,
// Ref::map, and a simple observer pattern with Weak subscribers that get dropped.
use std::cell::{Ref, RefCell, RefMut};
use std::collections::BTreeSet;
use std::rc::{Rc, Weak};

#[derive(Debug)]
struct Node {
    id: u32,
    value: i64,
    out: Vec<Rc<RefCell<Node>>>,
    back: Vec<Weak<RefCell<Node>>>,
}

type NodeRef = Rc<RefCell<Node>>;

fn node(id: u32, value: i64) -> NodeRef {
    Rc::new(RefCell::new(Node { id, value, out: vec![], back: vec![] }))
}

fn link(a: &NodeRef, b: &NodeRef) {
    a.borrow_mut().out.push(Rc::clone(b));
    b.borrow_mut().back.push(Rc::downgrade(a));
}

fn reachable_sum(start: &NodeRef) -> (i64, Vec<u32>) {
    let mut seen = BTreeSet::new();
    let mut stack = vec![Rc::clone(start)];
    let mut sum = 0;
    while let Some(n) = stack.pop() {
        let nb = n.borrow();
        if !seen.insert(nb.id) {
            continue;
        }
        sum += nb.value;
        for o in nb.out.iter().rev() {
            stack.push(Rc::clone(o));
        }
    }
    (sum, seen.into_iter().collect())
}

trait Observer {
    fn notify(&self, ev: &str);
}
struct Recorder {
    name: String,
    got: RefCell<Vec<String>>,
}
impl Observer for Recorder {
    fn notify(&self, ev: &str) {
        self.got.borrow_mut().push(format!("{}<-{}", self.name, ev));
    }
}
struct Subject {
    subs: RefCell<Vec<Weak<dyn Observer>>>,
}
impl Subject {
    fn emit(&self, ev: &str) -> usize {
        let mut delivered = 0;
        self.subs.borrow_mut().retain(|w| match w.upgrade() {
            Some(o) => {
                o.notify(ev);
                delivered += 1;
                true
            }
            None => false,
        });
        delivered
    }
}

fn main() {
    let a = node(1, 10);
    let b = node(2, 20);
    let c = node(3, 30);
    let d = node(4, 40);
    link(&a, &b);
    link(&a, &c);
    link(&b, &d);
    link(&c, &d);
    link(&d, &b);
    println!("{:?}", reachable_sum(&a));
    println!("{:?}", reachable_sum(&c));
    println!("counts a={} b={} d={}", Rc::strong_count(&a), Rc::strong_count(&b), Rc::strong_count(&d));
    let parents: Vec<u32> = d.borrow().back.iter().filter_map(|w| w.upgrade()).map(|p| p.borrow().id).collect();
    println!("parents of d {:?}", parents);

    for n in [&a, &b, &c, &d] {
        let mut nm = n.borrow_mut();
        let extra: i64 = nm.out.iter().map(|o| o.try_borrow().map(|x| x.value).unwrap_or(-1000)).sum();
        nm.value += extra;
    }
    println!("{:?}", [&a, &b, &c, &d].iter().map(|n| n.borrow().value).collect::<Vec<_>>());

    {
        let _hold = b.borrow_mut();
        println!("try_borrow while mut: {} {}", b.try_borrow().is_err(), b.try_borrow_mut().is_err());
    }
    {
        let r1 = b.borrow();
        let r2 = b.borrow();
        println!("two shared ok: {} {} try_mut err {}", r1.id, r2.id, b.try_borrow_mut().is_err());
    }

    let val_ref: Ref<i64> = Ref::map(c.borrow(), |n| &n.value);
    println!("mapped ref {}", *val_ref);
    drop(val_ref);
    {
        let mut vm: RefMut<i64> = RefMut::map(c.borrow_mut(), |n| &mut n.value);
        *vm *= 2;
    }
    println!("c value {}", c.borrow().value);

    let cell = RefCell::new(vec![1, 2]);
    let old = cell.replace(vec![9]);
    let other = RefCell::new(vec![7, 7]);
    cell.swap(&other);
    let taken = other.take();
    println!("{:?} {:?} {:?} {:?}", old, cell.borrow(), taken, other.borrow());
    cell.replace_with(|v| v.iter().map(|x| x + 1).collect());
    println!("{:?}", cell.into_inner());

    let subject = Subject { subs: RefCell::new(vec![]) };
    let r1: Rc<Recorder> = Rc::new(Recorder { name: "r1".into(), got: RefCell::new(vec![]) });
    let keep;
    {
        let r2: Rc<Recorder> = Rc::new(Recorder { name: "r2".into(), got: RefCell::new(vec![]) });
        let w1: Weak<dyn Observer> = Rc::downgrade(&r1) as Weak<dyn Observer>;
        let w2: Weak<dyn Observer> = Rc::downgrade(&r2) as Weak<dyn Observer>;
        subject.subs.borrow_mut().push(w1);
        subject.subs.borrow_mut().push(w2);
        println!("delivered {}", subject.emit("first"));
        keep = r2.got.borrow().clone();
    }
    println!("delivered {}", subject.emit("second"));
    println!("subs left {}", subject.subs.borrow().len());
    println!("{:?} {:?}", r1.got.borrow(), keep);

    // break cycles before exit
    for n in [&a, &b, &c, &d] {
        n.borrow_mut().out.clear();
    }
    println!("after clear counts a={} b={} d={}", Rc::strong_count(&a), Rc::strong_count(&b), Rc::strong_count(&d));
}
