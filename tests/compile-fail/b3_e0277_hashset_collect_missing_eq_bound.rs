//~ ERROR E0277
use std::collections::HashSet;
use std::hash::Hash;
fn uniq<T: Hash>(v: Vec<T>) -> usize {
    let s: HashSet<T> = v.into_iter().collect();
    s.len()
}
fn main() { uniq(vec![1]); }
