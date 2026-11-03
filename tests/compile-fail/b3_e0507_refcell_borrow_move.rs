//~ ERROR E0507
use std::cell::RefCell;
fn main() {
    let c = RefCell::new(vec![1, 2]);
    let v: Vec<i32> = *c.borrow();
    println!("{:?}", v);
}
