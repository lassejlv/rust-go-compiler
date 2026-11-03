//~ ERROR E0599
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<i32, i32> = HashMap::new();
    for v in m.value_mut() { *v += 1; }
}
