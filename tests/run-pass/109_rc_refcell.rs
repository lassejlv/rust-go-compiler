use std::cell::{Cell, RefCell};
use std::collections::HashMap;
use std::rc::{Rc, Weak};

#[derive(Debug)]
struct Account {
    owner: String,
    balance: i64,
}

struct Bank {
    accounts: Vec<Rc<RefCell<Account>>>,
}

#[derive(Debug)]
struct TreeNode {
    name: String,
    children: RefCell<Vec<Rc<TreeNode>>>,
    parent: RefCell<Weak<TreeNode>>,
}

fn new_node(name: &str) -> Rc<TreeNode> {
    Rc::new(TreeNode { name: name.to_string(), children: RefCell::new(vec![]), parent: RefCell::new(Weak::new()) })
}

fn add_child(parent: &Rc<TreeNode>, child: Rc<TreeNode>) {
    *child.parent.borrow_mut() = Rc::downgrade(parent);
    parent.children.borrow_mut().push(child);
}

fn path(node: &Rc<TreeNode>) -> String {
    match node.parent.borrow().upgrade() {
        Some(p) => format!("{}/{}", path(&p), node.name),
        None => node.name.clone(),
    }
}

fn main() {
    let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
    let a = Rc::clone(&shared);
    let b = Rc::clone(&shared);
    a.borrow_mut().push(4);
    b.borrow_mut().push(5);
    println!("shared = {:?}, count = {}", shared.borrow(), Rc::strong_count(&shared));
    {
        let r1 = shared.borrow();
        let r2 = shared.borrow();
        println!("two immutable borrows: {} {}", r1.len(), r2[0]);
    }
    shared.borrow_mut().retain(|x| x % 2 == 1);
    println!("after retain = {:?}", a.borrow());

    let alice = Rc::new(RefCell::new(Account { owner: "alice".into(), balance: 100 }));
    let bob = Rc::new(RefCell::new(Account { owner: "bob".into(), balance: 50 }));
    let bank = Bank { accounts: vec![alice.clone(), bob.clone()] };
    let transfer = |from: &Rc<RefCell<Account>>, to: &Rc<RefCell<Account>>, amt: i64| -> bool {
        if from.borrow().balance < amt {
            return false;
        }
        from.borrow_mut().balance -= amt;
        to.borrow_mut().balance += amt;
        true
    };
    println!("transfer 30: {}", transfer(&alice, &bob, 30));
    println!("transfer 500: {}", transfer(&bob, &alice, 500));
    alice.borrow_mut().balance += 1;
    for acc in &bank.accounts {
        let a = acc.borrow();
        println!("  {} has {}", a.owner, a.balance);
    }
    let total: i64 = bank.accounts.iter().map(|a| a.borrow().balance).sum();
    println!("total = {}", total);

    let counter = Cell::new(0);
    let bump = || counter.set(counter.get() + 1);
    bump();
    bump();
    bump();
    println!("cell counter = {}", counter.get());
    println!("cell replace old = {}, new = {}", counter.replace(100), counter.get());

    let root = new_node("root");
    let usr = new_node("usr");
    let bin = new_node("bin");
    let local = new_node("local");
    add_child(&root, usr.clone());
    add_child(&root, bin.clone());
    add_child(&usr, local.clone());
    println!("path = {}", path(&local));
    println!("path = {}", path(&bin));
    println!("root children = {:?}", root.children.borrow().iter().map(|c| c.name.clone()).collect::<Vec<_>>());
    println!("usr strong = {}, weak = {}", Rc::strong_count(&usr), Rc::weak_count(&usr));
    let orphan_parent = {
        let tmp = new_node("tmp");
        let child = new_node("child");
        add_child(&tmp, child.clone());
        child
    };
    println!("orphan parent alive: {}", orphan_parent.parent.borrow().upgrade().is_some());

    let memo: RefCell<HashMap<u64, u64>> = RefCell::new(HashMap::new());
    fn fib(n: u64, memo: &RefCell<HashMap<u64, u64>>) -> u64 {
        if n < 2 {
            return n;
        }
        if let Some(&v) = memo.borrow().get(&n) {
            return v;
        }
        let v = fib(n - 1, memo) + fib(n - 2, memo);
        memo.borrow_mut().insert(n, v);
        v
    }
    println!("fib(80) = {}", fib(80, &memo));
    println!("memo size = {}", memo.borrow().len());
    println!("try_borrow_mut while borrowed: {}", {
        let _g = shared.borrow();
        shared.try_borrow_mut().is_err()
    });
}
