//~ ERROR E0596
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<u32, Vec<u32>> = HashMap::new();
    m.insert(1, vec![]);
    if let Some(v) = m.get(&1) { v.push(2); }
}
