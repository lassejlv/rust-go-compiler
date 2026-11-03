use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::rc::{Rc, Weak};

struct Node {
    id: u32,
    label: String,
    edges: RefCell<Vec<Rc<Node>>>,
    back: RefCell<Vec<Weak<Node>>>,
    visits: Cell<u32>,
}

impl Node {
    fn new(id: u32, label: &str) -> Rc<Node> {
        Rc::new(Node { id, label: label.to_string(), edges: RefCell::new(vec![]), back: RefCell::new(vec![]), visits: Cell::new(0) })
    }
}

impl Drop for Node {
    fn drop(&mut self) { println!("drop node {} ({})", self.id, self.label); }
}

fn connect(a: &Rc<Node>, b: &Rc<Node>) {
    a.edges.borrow_mut().push(Rc::clone(b));
    b.back.borrow_mut().push(Rc::downgrade(a));
}

fn dfs(n: &Rc<Node>, depth: usize, out: &mut Vec<String>) {
    n.visits.set(n.visits.get() + 1);
    if n.visits.get() > 1 { out.push(format!("{}{}*", "  ".repeat(depth), n.label)); return; }
    out.push(format!("{}{}", "  ".repeat(depth), n.label));
    for e in n.edges.borrow().iter() { dfs(e, depth + 1, out); }
}

#[derive(Debug)]
struct TreeNode {
    val: i32,
    parent: RefCell<Weak<TreeNode>>,
    children: RefCell<Vec<Rc<TreeNode>>>,
}

fn add_child(parent: &Rc<TreeNode>, val: i32) -> Rc<TreeNode> {
    let c = Rc::new(TreeNode { val, parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    *c.parent.borrow_mut() = Rc::downgrade(parent);
    parent.children.borrow_mut().push(Rc::clone(&c));
    c
}

fn path_to_root(n: &Rc<TreeNode>) -> Vec<i32> {
    let mut v = vec![n.val];
    let mut cur = n.parent.borrow().upgrade();
    while let Some(p) = cur {
        v.push(p.val);
        cur = p.parent.borrow().upgrade();
    }
    v
}

fn subtree_sum(n: &TreeNode) -> i32 { n.val + n.children.borrow().iter().map(|c| subtree_sum(c)).sum::<i32>() }

struct Account { balance: RefCell<i64>, log: RefCell<Vec<String>> }

fn transfer(from: &Account, to: &Account, amt: i64) -> Result<(), String> {
    {
        let mut fb = from.balance.borrow_mut();
        if *fb < amt { return Err(format!("insufficient: have {} need {}", *fb, amt)); }
        *fb -= amt;
    }
    *to.balance.borrow_mut() += amt;
    from.log.borrow_mut().push(format!("-{}", amt));
    to.log.borrow_mut().push(format!("+{}", amt));
    Ok(())
}

fn main() {
    {
        let a = Node::new(1, "a");
        let b = Node::new(2, "b");
        let c = Node::new(3, "c");
        let d = Node::new(4, "d");
        connect(&a, &b); connect(&a, &c); connect(&b, &d); connect(&c, &d);
        println!("rc counts a={} b={} c={} d={}", Rc::strong_count(&a), Rc::strong_count(&b), Rc::strong_count(&c), Rc::strong_count(&d));
        println!("weak counts a={} d={}", Rc::weak_count(&a), Rc::weak_count(&d));
        let mut out = vec![];
        dfs(&a, 0, &mut out);
        println!("{}", out.join("\n"));
        let parents: Vec<String> = d.back.borrow().iter().filter_map(|w| w.upgrade()).map(|n| n.label.clone()).collect();
        println!("d's parents {:?}", parents);
        drop(a);
        let parents2: Vec<Option<u32>> = b.back.borrow().iter().map(|w| w.upgrade().map(|n| n.id)).collect();
        println!("b's parents after dropping a: {:?}", parents2);
        println!("end of scope");
    }
    println!("--- tree");
    let root = Rc::new(TreeNode { val: 1, parent: RefCell::new(Weak::new()), children: RefCell::new(vec![]) });
    let c1 = add_child(&root, 2);
    let c2 = add_child(&root, 3);
    let g1 = add_child(&c1, 4);
    let g2 = add_child(&c2, 5);
    let gg = add_child(&g2, 6);
    println!("path {:?} {:?} {:?}", path_to_root(&gg), path_to_root(&g1), path_to_root(&root));
    println!("sum root {} c2 {}", subtree_sum(&root), subtree_sum(&c2));
    println!("strong(c1)={} weak(c1)={} strong(root)={} weak(root)={}", Rc::strong_count(&c1), Rc::weak_count(&c1), Rc::strong_count(&root), Rc::weak_count(&root));
    let weak_root = Rc::downgrade(&root);
    drop(root);
    println!("root alive? {} gg parent chain {:?}", weak_root.upgrade().is_some(), path_to_root(&gg));
    drop(c1); drop(c2);
    println!("g1 parent alive? {}", g1.parent.borrow().upgrade().is_some());

    println!("--- accounts");
    let x = Account { balance: RefCell::new(100), log: RefCell::new(vec![]) };
    let y = Account { balance: RefCell::new(5), log: RefCell::new(vec![]) };
    for amt in [30, 80, 70, 1] {
        match transfer(&x, &y, amt) { Ok(()) => println!("ok {}", amt), Err(e) => println!("err {}", e) }
    }
    match transfer(&y, &x, 106) { Ok(()) => println!("ok back"), Err(e) => println!("err {}", e) }
    println!("x={} {:?} y={} {:?}", x.balance.borrow(), x.log.borrow(), y.balance.borrow(), y.log.borrow());
    let r1 = x.balance.borrow();
    println!("try_borrow_mut while borrowed: {}", x.balance.try_borrow_mut().is_err());
    drop(r1);
    println!("try_borrow_mut after drop: {}", x.balance.try_borrow_mut().is_ok());

    let shared: Rc<RefCell<BTreeMap<&str, i32>>> = Rc::new(RefCell::new(BTreeMap::new()));
    let handles: Vec<Rc<RefCell<BTreeMap<&str, i32>>>> = (0..3).map(|_| Rc::clone(&shared)).collect();
    for (i, h) in handles.iter().enumerate() {
        h.borrow_mut().insert(["x", "y", "z"][i], i as i32 * 10);
        *h.borrow_mut().entry("x").or_insert(0) += 1;
    }
    println!("shared {:?} strong={}", shared.borrow(), Rc::strong_count(&shared));
    drop(handles);
    let inner = Rc::try_unwrap(shared).map(|c| c.into_inner());
    println!("unwrapped {:?}", inner);
    let c = Cell::new(5);
    let old = c.replace(9);
    c.set(c.get() * 2);
    println!("cell old={} now={} take={} after={}", old, c.get(), c.take(), c.get());
    let rc_s = Rc::new(String::from("cow"));
    let mut rc_t = Rc::clone(&rc_s);
    Rc::make_mut(&mut rc_t).push_str("boy");
    println!("{} {} ptr_eq={}", rc_s, rc_t, Rc::ptr_eq(&rc_s, &rc_t));
    println!("done");
}
