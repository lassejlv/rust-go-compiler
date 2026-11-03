//~ ERROR E0308
use std::collections::HashMap;
fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    if m.get("a") == 1 { println!("one"); }
}
