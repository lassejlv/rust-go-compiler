//~ ERROR E0308
trait Area {
    fn width(&self) -> f64;
    fn area(&self) -> f64 { self.width() * 2.0; 0 }
}
fn main() {}
