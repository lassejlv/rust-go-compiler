//~ ERROR E0308
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<String, Vec<u8>> = HashMap::new();
    m.insert("k".to_string(), &[1u8, 2][..]);
}
