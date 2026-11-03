// A mutable tree with Rc children and Weak parent links: path-to-root,
// reparenting, subtree removal with drop tracking, depth-first and
// breadth-first traversals, and strong/weak count bookkeeping.
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::rc::{Rc, Weak};

thread_local! { static DROPPED: RefCell<Vec<String>> = RefCell::new(vec![]); }

struct Node { name: String, size: Cell<u64>, parent: RefCell<Weak<Node>>, kids: RefCell<Vec<Rc<Node>>> }
impl Drop for Node { fn drop(&mut self) { DROPPED.with(|d| d.borrow_mut().push(self.name.clone())); } }

fn node(name: &str, size: u64) -> Rc<Node> {
    Rc::new(Node { name: name.into(), size: Cell::new(size), parent: RefCell::new(Weak::new()), kids: RefCell::new(vec![]) })
}
fn attach(parent: &Rc<Node>, child: &Rc<Node>) {
    if let Some(old) = child.parent.borrow().upgrade() { old.kids.borrow_mut().retain(|k| !Rc::ptr_eq(k, child)); }
    *child.parent.borrow_mut() = Rc::downgrade(parent);
    parent.kids.borrow_mut().push(Rc::clone(child));
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
fn total(n: &Node) -> u64 { n.size.get() + n.kids.borrow().iter().map(|k| total(k)).sum::<u64>() }
fn dfs(n: &Rc<Node>, depth: usize, out: &mut Vec<String>) {
    out.push(format!("{}{}({})", "  ".repeat(depth), n.name, total(n)));
    for k in n.kids.borrow().iter() { dfs(k, depth + 1, out); }
}
fn bfs(root: &Rc<Node>) -> Vec<String> {
    let mut q = VecDeque::from([(Rc::clone(root), 0)]);
    let mut out = vec![];
    while let Some((n, d)) = q.pop_front() {
        out.push(format!("{}@{}", n.name, d));
        for k in n.kids.borrow().iter() { q.push_back((Rc::clone(k), d + 1)); }
    }
    out
}
fn find(n: &Rc<Node>, name: &str) -> Option<Rc<Node>> {
    if n.name == name { return Some(Rc::clone(n)); }
    n.kids.borrow().iter().find_map(|k| find(k, name))
}
fn dropped() -> Vec<String> { DROPPED.with(|d| std::mem::take(&mut *d.borrow_mut())) }

fn main() {
    let root = node("root", 1);
    {
        let usr = node("usr", 2);
        let bin = node("bin", 10);
        let lib = node("lib", 20);
        let local = node("local", 3);
        let share = node("share", 5);
        attach(&root, &usr);
        attach(&root, &bin);
        attach(&usr, &lib);
        attach(&usr, &local);
        attach(&local, &share);
        for f in ["a.so", "b.so"] { attach(&lib, &node(f, 100)); }
        println!("path {} strong(share)={} weak(local)={}", path(&share), Rc::strong_count(&share), Rc::weak_count(&local));
    }
    let mut out = vec![];
    dfs(&root, 0, &mut out);
    println!("{}", out.join("\n"));
    println!("bfs {:?}", bfs(&root));

    let share = find(&root, "share").unwrap();
    let bin = find(&root, "bin").unwrap();
    attach(&bin, &share);
    println!("after reparent: {} total(root)={} total(usr)={}", path(&share), total(&root), total(&find(&root, "usr").unwrap()));
    drop(share);
    drop(bin);

    let lib = find(&root, "lib").unwrap();
    let weak_lib = Rc::downgrade(&lib);
    let a_so = find(&root, "a.so").unwrap();
    let usr = find(&root, "usr").unwrap();
    usr.kids.borrow_mut().retain(|k| k.name != "lib");
    println!("lib detached, still alive via local handle: {} strong={}", weak_lib.upgrade().is_some(), Rc::strong_count(&lib));
    println!("a.so parent still reachable: {:?}", a_so.parent.borrow().upgrade().map(|p| p.name.clone()));
    drop(lib);
    println!("dropped after releasing lib: {:?}", dropped());
    println!("a.so parent now: {:?} weak alive: {}", a_so.parent.borrow().upgrade().map(|p| p.name.clone()), weak_lib.upgrade().is_some());
    drop(a_so);
    println!("dropped after a.so: {:?}", dropped());
    drop(usr);
    out.clear();
    dfs(&root, 0, &mut out);
    println!("{}", out.join(" | "));
    println!("root counts strong={} weak={}", Rc::strong_count(&root), Rc::weak_count(&root));
    drop(root);
    let mut d = dropped();
    println!("final drops ({}): {:?}", d.len(), { d.sort(); d });
}
