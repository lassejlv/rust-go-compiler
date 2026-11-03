//~ ERROR E0596
use std::rc::Rc;
fn main() {
    let r = Rc::new(vec![1]);
    r.push(2);
}
