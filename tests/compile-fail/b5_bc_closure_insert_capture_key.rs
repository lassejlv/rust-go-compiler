//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<&str, i32> = HashMap::new();
    {
        let k = String::from("k");
        let mut f = || {
            m.insert(&k, 1);
        };
        f();
    }
    println!("{}", m.len());
}
