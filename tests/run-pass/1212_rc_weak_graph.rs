use std::cell::{Cell, RefCell};
use std::rc::{Rc, Weak};

#[derive(Debug)]
struct Node {
    id: u32,
    children: RefCell<Vec<Rc<Node>>>,
    parent: RefCell<Weak<Node>>,
    visits: Cell<u32>,
}

impl Node {
    fn new(id: u32) -> Rc<Node> {
        Rc::new(Node { id, children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()), visits: Cell::new(0) })
    }
    fn add_child(self: &Rc<Self>, c: Rc<Node>) {
        *c.parent.borrow_mut() = Rc::downgrade(self);
        self.children.borrow_mut().push(c);
    }
    fn path(&self) -> Vec<u32> {
        let mut p = vec![self.id];
        let mut cur = self.parent.borrow().upgrade();
        while let Some(n) = cur {
            n.visits.set(n.visits.get() + 1);
            p.push(n.id);
            cur = n.parent.borrow().upgrade();
        }
        p
    }
    fn sum(&self) -> u32 {
        self.id + self.children.borrow().iter().map(|c| c.sum()).sum::<u32>()
    }
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop node {}", self.id);
    }
}

struct Observer {
    name: String,
    seen: RefCell<Vec<i32>>,
}

struct Subject {
    observers: Vec<Weak<Observer>>,
}

impl Subject {
    fn notify(&mut self, v: i32) -> usize {
        self.observers.retain(|w| w.strong_count() > 0);
        for o in self.observers.iter().filter_map(|w| w.upgrade()) {
            o.seen.borrow_mut().push(v);
        }
        self.observers.len()
    }
}

#[derive(Debug)]
struct DNode {
    val: i32,
    next: Option<Rc<RefCell<DNode>>>,
    prev: Option<Weak<RefCell<DNode>>>,
}

fn main() {
    let root = Node::new(1);
    {
        let a = Node::new(2);
        let b = Node::new(3);
        let c = Node::new(4);
        b.add_child(c.clone());
        root.add_child(a.clone());
        root.add_child(b.clone());
        println!("path of c: {:?}", c.path());
        println!("counts: root={} weak={} b={} c={}", Rc::strong_count(&root), Rc::weak_count(&root), Rc::strong_count(&b), Rc::strong_count(&c));
    }
    println!("sum={} visits root={}", root.sum(), root.visits.get());
    let grand = root.children.borrow()[1].children.borrow()[0].clone();
    println!("grand parent id: {:?}", grand.parent.borrow().upgrade().map(|p| p.id));
    let removed = root.children.borrow_mut().remove(1);
    println!("removed {} strong={}", removed.id, Rc::strong_count(&removed));
    drop(removed);
    println!("grand parent after drop: {:?}", grand.parent.borrow().upgrade().map(|p| p.id));
    drop(grand);
    println!("--");

    let o1 = Rc::new(Observer { name: "o1".into(), seen: RefCell::new(vec![]) });
    let mut subj = Subject { observers: vec![] };
    subj.observers.push(Rc::downgrade(&o1));
    {
        let o2 = Rc::new(Observer { name: "o2".into(), seen: RefCell::new(vec![]) });
        subj.observers.push(Rc::downgrade(&o2));
        println!("live {}", subj.notify(1));
        println!("{} {:?}", o2.name, o2.seen.borrow());
    }
    println!("live {}", subj.notify(2));
    println!("{} {:?}", o1.name, o1.seen.borrow());

    // doubly linked list
    let n1 = Rc::new(RefCell::new(DNode { val: 1, next: None, prev: None }));
    let n2 = Rc::new(RefCell::new(DNode { val: 2, next: None, prev: None }));
    let n3 = Rc::new(RefCell::new(DNode { val: 3, next: None, prev: None }));
    n1.borrow_mut().next = Some(n2.clone());
    n2.borrow_mut().prev = Some(Rc::downgrade(&n1));
    n2.borrow_mut().next = Some(n3.clone());
    n3.borrow_mut().prev = Some(Rc::downgrade(&n2));
    let mut cur = Some(n1.clone());
    let mut fwd = vec![];
    while let Some(n) = cur {
        fwd.push(n.borrow().val);
        cur = n.borrow().next.clone();
    }
    let mut back = vec![];
    let mut cur = Some(n3.clone());
    while let Some(n) = cur {
        back.push(n.borrow().val);
        n.borrow_mut().val *= 10;
        cur = n.borrow().prev.as_ref().and_then(|w| w.upgrade());
    }
    println!("{:?} {:?} {}", fwd, back, n2.borrow().val);

    // Rc utilities
    let mut r = Rc::new(vec![1, 2]);
    Rc::make_mut(&mut r).push(3);
    let r2 = r.clone();
    Rc::make_mut(&mut r).push(4);
    println!("{:?} {:?} {}", r, r2, Rc::ptr_eq(&r, &r2));
    println!("{:?}", Rc::try_unwrap(r2));
    let shared = Rc::new(5);
    let s2 = Rc::clone(&shared);
    println!("{:?}", Rc::try_unwrap(shared).map_err(|e| *e + 100));
    println!("{:?}", Rc::get_mut(&mut s2.clone()).is_some());
    let mut uniq = Rc::new(String::from("u"));
    Rc::get_mut(&mut uniq).unwrap().push('!');
    println!("{} {}", uniq, Rc::into_inner(uniq.clone()).is_none());
    let w: Weak<i32> = Weak::new();
    println!("{:?} {}", w.upgrade(), w.strong_count());
    let rc_cell = Rc::new(Cell::new(1));
    let rc_cell2 = rc_cell.clone();
    rc_cell2.set(rc_cell.get() + 41);
    println!("{} {}", rc_cell.get(), rc_cell.replace(0));
    let rcell = RefCell::new(vec![1]);
    {
        let b1 = rcell.borrow();
        println!("{:?} {}", rcell.try_borrow_mut().is_err(), b1.len());
    }
    rcell.borrow_mut().push(2);
    let old = rcell.replace(vec![9]);
    println!("{:?} {:?}", old, rcell.take());
    let rm = RefCell::new((1, 2));
    {
        let mut g = rm.borrow_mut();
        g.0 += 10;
    }
    let mapped = std::cell::Ref::map(rm.borrow(), |t| &t.1);
    println!("{} {:?}", *mapped, rm);
    drop(mapped);
    println!("end");
}
