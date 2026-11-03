//~ ERROR E0186
trait Shape { fn area(&self) -> f64; }
struct Sq;
impl Shape for Sq { fn area() -> f64 { 1.0 } }
fn main() {}
