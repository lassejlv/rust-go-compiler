//~ ERROR E0449
// So are the fields of enum variants.
pub enum Shape {
    Circle { pub radius: f64 },
    Square(f64),
}
fn main() {
    let _ = (Shape::Circle { radius: 1.0 }, Shape::Square(2.0));
}
