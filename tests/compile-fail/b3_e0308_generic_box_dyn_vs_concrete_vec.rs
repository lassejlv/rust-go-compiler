//~ ERROR E0308
trait Shape {}
struct Sq;
impl Shape for Sq {}
fn main() {
    let shapes: Vec<Box<dyn Shape>> = vec![Sq];
    let _ = shapes;
}
