//~ ERROR E0277
use std::collections::BTreeMap;
#[derive(PartialEq, Eq)]
struct Id(u32);
fn main() {
    let mut m = BTreeMap::new();
    m.insert(Id(1), "x");
}
