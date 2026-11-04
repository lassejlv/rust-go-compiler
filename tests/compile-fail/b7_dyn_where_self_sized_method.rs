//~ ERROR cannot be invoked on a trait object
// A method with `where Self: Sized` is not callable on a trait object.
trait Shape {
    fn area(&self) -> f64;
    fn scaled(&self) -> f64 where Self: Sized {
        self.area() * 2.0
    }
}
struct Sq(f64);
impl Shape for Sq {
    fn area(&self) -> f64 {
        self.0 * self.0
    }
}
fn main() {
    let s: Box<dyn Shape> = Box::new(Sq(2.0));
    println!("{}", s.scaled());
}
