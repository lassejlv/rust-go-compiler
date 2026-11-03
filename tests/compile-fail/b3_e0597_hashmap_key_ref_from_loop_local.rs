//~ ERROR E0597
use std::collections::HashMap;
fn main() {
    let mut counts: HashMap<&str, usize> = HashMap::new();
    for line in ["a b", "c"] {
        let owned = line.to_uppercase();
        *counts.entry(&owned).or_default() += 1;
    }
    println!("{:?}", counts);
}
