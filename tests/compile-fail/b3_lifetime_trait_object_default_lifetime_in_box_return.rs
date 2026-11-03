//~ ERROR lifetime
trait Shape { fn area(&self) -> f64; }
struct Sq<'a>(&'a f64);
impl<'a> Shape for Sq<'a> { fn area(&self) -> f64 { *self.0 * *self.0 } }
fn make<'a>(x: &'a f64) -> Box<dyn Shape> { Box::new(Sq(x)) }
fn main() { let x = 2.0; make(&x).area(); }
