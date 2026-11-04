//~ ERROR E0658
trait Shape {
    type Unit: std::fmt::Debug + Default = f64;
    fn area(&self) -> Self::Unit;
}
struct Sq;
impl Shape for Sq { type Unit = f64; fn area(&self) -> f64 { 1.0 } }
fn main() { println!("{:?}", Sq.area()); }
