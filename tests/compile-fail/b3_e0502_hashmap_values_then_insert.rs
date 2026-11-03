//~ ERROR E0502
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 1);
    for v in m.values() {
        m.insert("b", *v);
    }
}
