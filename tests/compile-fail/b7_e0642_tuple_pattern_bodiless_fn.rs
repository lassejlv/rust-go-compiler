//~ ERROR E0642
// A tuple pattern in a required trait method.
trait Geometry {
    fn dist((x, y): (f64, f64)) -> f64;
}
fn main() {}
