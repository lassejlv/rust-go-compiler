//~ ERROR E0597
use std::collections::HashMap;

fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    {
        let key = String::from("k");
        m.insert(&key, 1);
    }
    println!("{}", m.len());
}
