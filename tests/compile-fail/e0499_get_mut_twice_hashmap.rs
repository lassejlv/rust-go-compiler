//~ ERROR E0499
use std::collections::HashMap;

fn main() {
    let mut m: HashMap<i32, i32> = HashMap::new();
    m.insert(1, 1);
    m.insert(2, 2);
    let a = m.get_mut(&1).unwrap();
    let b = m.get_mut(&2).unwrap();
    std::mem::swap(a, b);
}
