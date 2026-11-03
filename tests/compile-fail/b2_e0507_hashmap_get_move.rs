//~ ERROR E0507
use std::collections::HashMap;
fn main() {
    let mut m = HashMap::new();
    m.insert(1, String::from("a"));
    let s: String = *m.get(&1).unwrap();
    println!("{}", s);
}
