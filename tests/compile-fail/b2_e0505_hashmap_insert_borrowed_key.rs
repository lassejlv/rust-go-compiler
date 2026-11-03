//~ ERROR E0505
use std::collections::HashMap;
fn main() {
    let k = String::from("key");
    let kr: &str = &k;
    let mut m = HashMap::new();
    m.insert(k, 1);
    println!("{}", kr);
}
