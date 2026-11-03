//~ ERROR E0502
use std::collections::HashMap;
fn get_or_default<'a>(m: &'a mut HashMap<String, String>, k: &str) -> &'a String {
    match m.get(k) {
        Some(v) => v,
        None => {
            m.insert(k.to_string(), String::new());
            m.get(k).unwrap()
        }
    }
}
fn main() { let mut m = HashMap::new(); println!("{}", get_or_default(&mut m, "a")); }
