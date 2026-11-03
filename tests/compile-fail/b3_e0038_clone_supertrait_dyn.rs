//~ ERROR E0038
trait Shape: Clone { fn area(&self) -> f64; }
fn total(v: Vec<Box<dyn Shape>>) -> f64 { v.iter().map(|s| s.area()).sum() }
fn main() {}
