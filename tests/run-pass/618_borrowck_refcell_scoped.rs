// RefCell borrow guards that are released before the next borrow_mut.
use std::cell::RefCell;
use std::rc::Rc;

fn main() {
    let shared = Rc::new(RefCell::new(vec![1, 2, 3]));
    let other = Rc::clone(&shared);
    {
        let mut g = shared.borrow_mut();
        g.push(4);
    }
    let len = other.borrow().len();
    other.borrow_mut().push(len as i32);
    let sum: i32 = shared.borrow().iter().sum();
    shared.borrow_mut().push(sum);
    let snapshot = shared.borrow().clone();
    shared.borrow_mut().clear();
    let c = RefCell::new(String::from("x"));
    let n = c.borrow().len();
    c.borrow_mut().push_str(&"y".repeat(n));
    println!("{:?} {:?} {}", snapshot, other.borrow(), c.borrow());
}
