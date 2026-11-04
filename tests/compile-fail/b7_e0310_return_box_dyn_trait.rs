//~ ERROR E0310
// A returned `Box<dyn Trait>` is `+ 'static` by default.
use std::fmt::Debug;
fn boxed<T: Debug>(x: T) -> Box<dyn Debug> {
    Box::new(x)
}
fn main() {
    println!("{:?}", boxed(1));
}
