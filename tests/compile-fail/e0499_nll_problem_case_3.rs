//~ ERROR E0499
use std::collections::HashMap;

fn get_default(map: &mut HashMap<i32, String>, key: i32) -> &mut String {
    match map.get_mut(&key) {
        Some(v) => v,
        None => {
            map.insert(key, String::new());
            map.get_mut(&key).unwrap()
        }
    }
}

fn main() {
    let mut m = HashMap::new();
    get_default(&mut m, 1).push('a');
}
