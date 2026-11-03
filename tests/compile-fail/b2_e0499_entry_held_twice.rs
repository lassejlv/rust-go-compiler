//~ ERROR E0499
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    let e1 = m.entry("a").or_insert(0);
    let e2 = m.entry("b").or_insert(0);
    *e1 += 1;
    *e2 += 1;
}
