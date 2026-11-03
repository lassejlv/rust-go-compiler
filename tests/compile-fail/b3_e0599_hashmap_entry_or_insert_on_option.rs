//~ ERROR E0599
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    *m.get_mut("a").or_insert(0) += 1;
}
