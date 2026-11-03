//~ ERROR E0596
use std::collections::HashMap;
fn bump(m: &HashMap<String, Vec<i32>>) {
    if let Some(v) = m.get("a") {
        v.push(1);
    }
}
fn main() { bump(&HashMap::new()); }
