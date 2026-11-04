//~ ERROR E0621
use std::collections::HashMap;
fn f<'a>(m: &'a HashMap<&'static str, String>, k: &str) -> Option<&'a String> {
    m.get(&k)
}
fn main() {
    let m = HashMap::new();
    println!("{:?}", f(&m, "x"));
}
