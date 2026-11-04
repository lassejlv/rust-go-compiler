//~ ERROR recursion limit
use std::fmt::Debug;
fn f<T: Debug>(n: u32, x: T) {
    if n > 0 { f(n - 1, (x,)) } else { println!("{:?}", x) }
}
fn main() { f(3, 1u8); }
