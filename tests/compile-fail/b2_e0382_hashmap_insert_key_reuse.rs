//~ ERROR E0382
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<String, i32> = HashMap::new();
    let key = String::from("k");
    m.insert(key, 1);
    println!("{:?}", m.get(&key));
}
