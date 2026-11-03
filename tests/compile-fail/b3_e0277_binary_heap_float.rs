//~ ERROR E0277
use std::collections::BinaryHeap;
fn main() {
    let mut h = BinaryHeap::new();
    h.push(1.5f64);
}
