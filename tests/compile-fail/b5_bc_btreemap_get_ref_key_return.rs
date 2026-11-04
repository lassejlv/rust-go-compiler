//~ ERROR E0515
use std::collections::BTreeMap;
// K = &'static str, so the key must be a `&&'a str` (Q = K): the result,
// returned as `&'a String`, keeps the local key borrowed
fn f<'a>(m: &'a BTreeMap<&'static str, String>) -> Option<&'a String> {
    let k = String::from("x");
    let kr: &str = &k;
    m.get(&kr)
}
fn main() {
    let m = BTreeMap::new();
    println!("{:?}", f(&m));
}
