//~ ERROR E0277
// Type parameters are implicitly `Sized`.
use std::fmt::Debug;
fn show<T: Debug>(x: &T) { println!("{:?}", x); }
fn main() {
    let d: &dyn Debug = &5;
    show(d);
}
