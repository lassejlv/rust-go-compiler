//~ ERROR E0015
// A const fn cannot call `Rc::new`.
use std::rc::Rc;
const fn shared(x: i32) -> Rc<i32> {
    Rc::new(x)
}
fn main() {
    println!("{}", shared(1));
}
