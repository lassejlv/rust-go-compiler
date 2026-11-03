//~ ERROR E0277
use std::collections::HashMap;
fn main() {
    let m: HashMap<i32, i32> = HashMap::new();
    for (k, v) in m.iter().rev() { println!("{} {}", k, v); }
}
