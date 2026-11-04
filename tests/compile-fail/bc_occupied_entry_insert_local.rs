//~ ERROR E0597
use std::collections::BTreeMap;
use std::collections::btree_map::Entry;
fn main() {
    let mut m: BTreeMap<u8, &str> = BTreeMap::new();
    m.insert(1, "one");
    {
        let s = String::from("x");
        if let Entry::Occupied(mut e) = m.entry(1) {
            e.insert(&s);
        }
    }
    println!("{:?}", m);
}
