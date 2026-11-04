//~ ERROR E0597
use std::collections::BTreeMap;
fn main() {
    let mut m: BTreeMap<&str, i32> = BTreeMap::new();
    {
        let s = String::from("a");
        if let Some(v) = m.get_mut(&s.as_str()) {
            *v += 1;
        }
    }
    println!("{}", m.len());
}
