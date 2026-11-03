//~ ERROR E0277
use std::collections::HashMap;
fn index<K>(keys: Vec<K>) -> usize {
    let mut m = HashMap::new();
    for k in keys { m.insert(k, ()); }
    m.len()
}
fn main() { index(vec![1]); }
