//~ ERROR E0574
enum Shape { Circle { r: f64 } }
fn main() { let s = Shape { r: 1.0 }; }
