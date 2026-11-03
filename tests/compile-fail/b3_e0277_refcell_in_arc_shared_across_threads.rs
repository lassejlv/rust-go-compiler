//~ ERROR E0277
use std::cell::RefCell;
use std::sync::Arc;
use std::thread;
fn main() {
    let a = Arc::new(RefCell::new(0));
    let b = Arc::clone(&a);
    thread::spawn(move || { *b.borrow_mut() += 1; }).join().unwrap();
}
