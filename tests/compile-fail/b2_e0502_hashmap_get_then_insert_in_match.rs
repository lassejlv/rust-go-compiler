//~ ERROR E0502
use std::collections::HashMap;
fn cache(m: &mut HashMap<u32, String>, k: u32) -> &String {
    match m.get(&k) {
        Some(v) => v,
        None => {
            m.insert(k, k.to_string());
            m.get(&k).unwrap()
        }
    }
}
fn main() { let mut m = HashMap::new(); println!("{}", cache(&mut m, 1)); }
