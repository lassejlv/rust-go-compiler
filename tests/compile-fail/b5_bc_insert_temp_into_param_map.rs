//~ ERROR E0716
use std::collections::HashMap;
fn f<'a>(m: &mut HashMap<&'a str, usize>) {
    m.insert(String::from("k").as_str(), 1);
}
fn main() {
    let mut m = HashMap::new();
    f(&mut m);
    println!("{}", m.len());
}
