//~ ERROR E0599
use std::rc::Rc;
fn main() {
    let r = Rc::new(vec![1]);
    r.borrow_mut().push(2);
}
