// Rc/Weak parent-child tree: parent pointers via Weak, upgrade after drop, strong/weak counts,
// a deliberate cycle broken manually, and Drop messages showing when nodes die.
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct Node {
    name: String,
    parent: RefCell<Weak<Node>>,
    children: RefCell<Vec<Rc<Node>>>,
}

impl Drop for Node {
    fn drop(&mut self) {
        println!("drop node {}", self.name);
    }
}

impl Node {
    fn new(name: &str) -> Rc<Node> {
        Rc::new(Node { name: name.to_string(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) })
    }
    fn add_child(parent: &Rc<Node>, child: Rc<Node>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(child);
    }
    fn path(&self) -> String {
        match self.parent.borrow().upgrade() {
            Some(p) => format!("{}/{}", p.path(), self.name),
            None => self.name.clone(),
        }
    }
    fn depth_sum(node: &Rc<Node>, depth: usize) -> usize {
        depth + node.children.borrow().iter().map(|c| Node::depth_sum(c, depth + 1)).sum::<usize>()
    }
}

struct Cyc {
    id: u32,
    next: RefCell<Option<Rc<Cyc>>>,
}
impl Drop for Cyc {
    fn drop(&mut self) {
        println!("drop cyc {}", self.id);
    }
}

fn main() {
    let leaf_weak: Weak<Node>;
    {
        let root = Node::new("root");
        let a = Node::new("a");
        let b = Node::new("b");
        let a1 = Node::new("a1");
        Node::add_child(&a, a1.clone());
        Node::add_child(&root, a.clone());
        Node::add_child(&root, b.clone());
        leaf_weak = Rc::downgrade(&a1);

        println!("path a1 = {}", a1.path());
        println!("path b = {}", b.path());
        println!("root strong={} weak={}", Rc::strong_count(&root), Rc::weak_count(&root));
        println!("a strong={} weak={}", Rc::strong_count(&a), Rc::weak_count(&a));
        println!("depth sum = {}", Node::depth_sum(&root, 0));

        drop(a1);
        println!("leaf alive? {}", leaf_weak.upgrade().is_some());

        // detach b from root
        let removed = root.children.borrow_mut().remove(1);
        *removed.parent.borrow_mut() = Weak::new();
        println!("removed {} path now {}", removed.name, removed.path());
        drop(removed);
        println!("b strong={}", Rc::strong_count(&b));
        drop(b);
        println!("end of scope");
    }
    println!("leaf alive after scope? {}", leaf_weak.upgrade().is_some());
    println!("leaf weak count = {}", leaf_weak.weak_count());

    // cycle
    let x = Rc::new(Cyc { id: 1, next: RefCell::new(None) });
    let y = Rc::new(Cyc { id: 2, next: RefCell::new(Some(x.clone())) });
    *x.next.borrow_mut() = Some(y.clone());
    println!("x strong={} y strong={}", Rc::strong_count(&x), Rc::strong_count(&y));
    let wx = Rc::downgrade(&x);
    drop(x);
    drop(y);
    println!("after drops, cycle alive: {}", wx.upgrade().is_some());
    // break the cycle
    if let Some(x) = wx.upgrade() {
        let taken = x.next.borrow_mut().take();
        println!("took next: {:?}", taken.as_ref().map(|c| c.id));
        drop(taken);
        println!("x strong now {}", Rc::strong_count(&x));
    }
    println!("cycle alive after break: {}", wx.upgrade().is_some());

    // Rc::try_unwrap and get_mut
    let mut shared = Rc::new(vec![1, 2, 3]);
    Rc::get_mut(&mut shared).unwrap().push(4);
    let other = shared.clone();
    println!("get_mut while shared: {}", Rc::get_mut(&mut shared).is_none());
    let shared = match Rc::try_unwrap(shared) {
        Ok(v) => {
            println!("unwrapped {:?}", v);
            Rc::new(v)
        }
        Err(rc) => {
            println!("still shared, count={}", Rc::strong_count(&rc));
            rc
        }
    };
    drop(other);
    let v = Rc::try_unwrap(shared).unwrap();
    println!("final {:?}", v);
    let mut cow = Rc::new(String::from("cow"));
    let cow2 = cow.clone();
    Rc::make_mut(&mut cow).push_str("-mut");
    println!("{} {} ptr_eq={}", cow, cow2, Rc::ptr_eq(&cow, &cow2));
}
