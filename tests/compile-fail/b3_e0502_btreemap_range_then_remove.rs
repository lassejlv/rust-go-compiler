//~ ERROR E0502
use std::collections::BTreeMap;
fn main() {
    let mut m = BTreeMap::new();
    m.insert(1, "a"); m.insert(2, "b");
    for (k, _) in m.range(..2) {
        m.remove(k);
    }
}
