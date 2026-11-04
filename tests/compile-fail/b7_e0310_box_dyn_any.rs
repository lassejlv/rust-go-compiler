//~ ERROR E0310
// Unsizing to `dyn Any` needs `T: 'static`.
use std::any::Any;
fn erase<T>(x: T) -> Box<dyn Any> {
    Box::new(x)
}
fn main() {
    println!("{}", erase(1).is::<i32>());
}
