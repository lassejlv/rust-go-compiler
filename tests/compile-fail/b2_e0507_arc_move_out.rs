//~ ERROR E0507
use std::sync::Arc;
fn main() {
    let a = Arc::new(String::from("x"));
    let s: String = *a;
    println!("{}", s);
}
