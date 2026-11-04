//~ ERROR lifetime
use std::collections::HashMap;
fn vals<'a>(m: &HashMap<&'static str, &'a str>) -> Vec<&'static str> {
    m.values().copied().collect()
}
fn main() {
    let s = String::from("v");
    let mut m = HashMap::new();
    m.insert("k", s.as_str());
    println!("{:?}", vals(&m));
}
