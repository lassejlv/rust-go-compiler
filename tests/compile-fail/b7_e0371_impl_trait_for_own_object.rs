//~ ERROR E0371
// `dyn Shape` already implements `Shape`.
trait Shape {
    fn area(&self) -> f64;
}
impl Shape for dyn Shape {
    fn area(&self) -> f64 {
        0.0
    }
}
fn main() {}
