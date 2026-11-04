//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<u8, &str> = HashMap::new();
    m.insert(1, "a");
    {
        let s = String::from("x");
        if let Some(x) = m.get_mut(&1) { *x = &s; }
        m.values_mut().for_each(|x| *x = &s);
    }
    println!("{:?}", m);
}
