//~ ERROR E0599
use std::collections::HashMap;
fn main() {
    let mut m: HashMap<f64, &str> = HashMap::new();
    m.insert(1.0, "one");
}
