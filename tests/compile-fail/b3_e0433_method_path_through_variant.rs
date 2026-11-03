//~ ERROR E0433
enum Shape { Circle(f64) }
impl Shape { fn area(&self) -> f64 { match self { Shape::Circle(r) => r * r } } }
fn main() { let a = Shape::Circle::area(1.0); }
