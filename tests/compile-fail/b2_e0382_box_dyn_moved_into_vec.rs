//~ ERROR E0382
trait Shape { fn area(&self) -> f64; }
struct Sq(f64);
impl Shape for Sq { fn area(&self) -> f64 { self.0 * self.0 } }
fn main() {
    let s: Box<dyn Shape> = Box::new(Sq(2.0));
    let mut shapes: Vec<Box<dyn Shape>> = vec![];
    shapes.push(s);
    println!("{}", s.area());
}
