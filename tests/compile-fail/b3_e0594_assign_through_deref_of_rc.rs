//~ ERROR E0594
use std::rc::Rc;
struct S { n: i32 }
fn main() {
    let r = Rc::new(S { n: 1 });
    r.n = 2;
}
