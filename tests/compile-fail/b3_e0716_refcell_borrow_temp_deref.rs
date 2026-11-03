//~ ERROR E0716
use std::cell::RefCell;
fn main() {
    let c = RefCell::new(vec![1, 2]);
    let first: &i32;
    first = c.borrow().first().unwrap();
    println!("{}", first);
}
