//~ ERROR E0277
// The same for a generic function's parameter type.
use std::fmt::Display;
struct Labeled<T: Display> {
    value: T,
}
fn show<T>(l: &Labeled<T>) -> usize {
    0
}
fn main() {}
