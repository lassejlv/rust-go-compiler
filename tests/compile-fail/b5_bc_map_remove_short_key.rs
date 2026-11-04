//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    m.insert("a", 1);
    {
        let s = String::from("a");
        m.remove(&s.as_str());
    }
    println!("{}", m.len());
}
