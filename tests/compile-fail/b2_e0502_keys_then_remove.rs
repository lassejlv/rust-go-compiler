//~ ERROR E0502
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<i32, i32> = (0..5).map(|i| (i, i)).collect();
    for k in m.keys() {
        if k % 2 == 0 { m.remove(k); }
    }
}
