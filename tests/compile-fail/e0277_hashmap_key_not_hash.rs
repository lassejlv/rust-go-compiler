//~ ERROR E0277
use std::collections::HashSet;

#[derive(PartialEq, Eq)]
struct K(i32);

fn main() {
    let mut s = HashSet::new();
    s.insert(K(1));
    println!("{}", s.len());
}
