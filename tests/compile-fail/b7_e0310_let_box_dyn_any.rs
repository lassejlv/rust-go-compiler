//~ ERROR E0310
// A `let` of type `Box<dyn Any>` too.
use std::any::Any;
fn store<T>(items: Vec<T>) -> usize {
    let b: Box<dyn Any> = Box::new(items);
    if b.is::<Vec<i32>>() { 1 } else { 0 }
}
fn main() {
    println!("{}", store(vec![1]));
}
