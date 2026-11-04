//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<u8, &str> = HashMap::new();
    {
        let s = String::from("x");
        m.entry(1).or_insert(&s);
    }
    println!("{:?}", m);
}
