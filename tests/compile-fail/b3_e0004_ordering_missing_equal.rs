//~ ERROR E0004
use std::cmp::Ordering;
fn f(a: i32, b: i32) -> &'static str {
    match a.cmp(&b) {
        Ordering::Less => "lt",
        Ordering::Greater => "gt",
    }
}
fn main() { f(1, 2); }
