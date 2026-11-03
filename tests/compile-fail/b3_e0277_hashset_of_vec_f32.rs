//~ ERROR E0277
use std::collections::HashSet;
fn main() {
    let mut s = HashSet::new();
    s.insert(vec![1.0f32]);
}
