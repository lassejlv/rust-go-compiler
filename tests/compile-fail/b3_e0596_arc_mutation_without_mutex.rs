//~ ERROR E0596
use std::sync::Arc;
fn main() {
    let a = Arc::new(String::from("x"));
    a.push_str("y");
}
