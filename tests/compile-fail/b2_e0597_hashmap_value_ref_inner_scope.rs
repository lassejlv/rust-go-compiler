//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<u32, &str> = HashMap::new();
    {
        let owned = String::from("v");
        m.insert(1, owned.as_str());
    }
    println!("{:?}", m);
}
