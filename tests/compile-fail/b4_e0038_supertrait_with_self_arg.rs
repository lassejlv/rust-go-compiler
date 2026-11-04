//~ ERROR E0038
trait Shape: PartialEq<Self> { fn area(&self) -> f64; }
fn total(shapes: &[Box<dyn Shape>]) -> f64 { shapes.iter().map(|s| s.area()).sum() }
fn main() {}
