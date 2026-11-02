// Rc<RefCell<>> parent/child trees with Weak back-pointers, cycle breaking, Drop tracing,
// and Rc::new_cyclic for self-referential handles.
use std::cell::RefCell;
use std::rc::{Rc, Weak};

struct TreeNode {
    name: String,
    parent: RefCell<Weak<TreeNode>>,
    children: RefCell<Vec<Rc<TreeNode>>>,
}

impl Drop for TreeNode {
    fn drop(&mut self) {
        println!("drop node {}", self.name);
    }
}

impl TreeNode {
    fn new(name: &str) -> Rc<TreeNode> {
        Rc::new(TreeNode { name: name.into(), parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) })
    }
    fn add_child(parent: &Rc<TreeNode>, child: Rc<TreeNode>) {
        *child.parent.borrow_mut() = Rc::downgrade(parent);
        parent.children.borrow_mut().push(child);
    }
    fn path(&self) -> String {
        match self.parent.borrow().upgrade() {
            Some(p) => format!("{}/{}", p.path(), self.name),
            None => self.name.clone(),
        }
    }
    fn depth_first(node: &Rc<TreeNode>, out: &mut Vec<String>) {
        out.push(node.path());
        for c in node.children.borrow().iter() {
            TreeNode::depth_first(c, out);
        }
    }
    fn detach(node: &Rc<TreeNode>) {
        if let Some(p) = node.parent.borrow().upgrade() {
            p.children.borrow_mut().retain(|c| !Rc::ptr_eq(c, node));
        }
        *node.parent.borrow_mut() = Weak::new();
    }
}

// Self-referential: a widget that can hand out weak handles to itself
struct Widget {
    id: u32,
    me: Weak<Widget>,
    clicks: RefCell<u32>,
}

impl Widget {
    fn new(id: u32) -> Rc<Widget> {
        Rc::new_cyclic(|me| Widget { id, me: me.clone(), clicks: RefCell::new(0) })
    }
    fn on_click_handler(&self) -> impl Fn() -> bool {
        let me = self.me.clone();
        move || match me.upgrade() {
            Some(w) => {
                *w.clicks.borrow_mut() += 1;
                true
            }
            None => false,
        }
    }
}

impl Drop for Widget {
    fn drop(&mut self) {
        println!("drop widget {} after {} clicks", self.id, self.clicks.borrow());
    }
}

// A deliberate strong cycle broken manually
struct Peer {
    name: &'static str,
    other: RefCell<Option<Rc<Peer>>>,
}
impl Drop for Peer {
    fn drop(&mut self) {
        println!("drop peer {}", self.name);
    }
}

fn main() {
    {
        let root = TreeNode::new("root");
        let src = TreeNode::new("src");
        let lib = TreeNode::new("lib.rs");
        let main_rs = TreeNode::new("main.rs");
        let docs = TreeNode::new("docs");
        TreeNode::add_child(&root, src.clone());
        TreeNode::add_child(&src, lib.clone());
        TreeNode::add_child(&src, main_rs);
        TreeNode::add_child(&root, docs);
        let mut out = vec![];
        TreeNode::depth_first(&root, &mut out);
        println!("{:?}", out);
        println!("src strong={} weak={}", Rc::strong_count(&src), Rc::weak_count(&src));
        println!("lib parent = {:?}", lib.parent.borrow().upgrade().map(|p| p.name.clone()));
        TreeNode::detach(&src);
        println!("after detach: lib path = {}", lib.path());
        let mut out = vec![];
        TreeNode::depth_first(&root, &mut out);
        println!("{:?}", out);
        drop(root);
        println!("root dropped; src still alive: {}", src.path());
        println!("leaving scope");
    }

    println!("-- widgets");
    let handler;
    {
        let w = Widget::new(7);
        let h = w.on_click_handler();
        println!("{} {} {}", h(), h(), h());
        handler = w.on_click_handler();
        println!("strong={} weak={}", Rc::strong_count(&w), Rc::weak_count(&w));
    }
    println!("handler after drop: {}", handler());

    println!("-- peers");
    let a = Rc::new(Peer { name: "a", other: RefCell::new(None) });
    let b = Rc::new(Peer { name: "b", other: RefCell::new(Some(a.clone())) });
    *a.other.borrow_mut() = Some(b.clone());
    println!("a strong={} b strong={}", Rc::strong_count(&a), Rc::strong_count(&b));
    // break the cycle
    a.other.borrow_mut().take();
    println!("cycle broken: a strong={} b strong={}", Rc::strong_count(&a), Rc::strong_count(&b));
    drop(b);
    println!("b handle dropped");
    drop(a);
    let w: Weak<Peer> = Weak::new();
    println!("dangling weak upgrade: {}", w.upgrade().is_none());
    println!("end");
}
