//~ ERROR E0502
use std::collections::HashMap;

fn main() {
    let mut m = HashMap::new();
    m.insert("a", 1);
    let a = m.get("a").unwrap();
    m.insert("b", *a + 1);
    println!("{}", a);
}
