//~ ERROR E0499
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<u32, Vec<u32>> = HashMap::new();
    m.insert(1, vec![]); m.insert(2, vec![]);
    let a = m.get_mut(&1).unwrap();
    let b = m.get_mut(&2).unwrap();
    a.push(1);
    b.push(2);
}
