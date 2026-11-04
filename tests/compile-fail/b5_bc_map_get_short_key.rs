//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let m: HashMap<&str, i32> = HashMap::from([("a", 1)]);
    let r;
    {
        let s = String::from("a");
        r = m.get(&s.as_str());
    }
    println!("{:?}", r);
}
