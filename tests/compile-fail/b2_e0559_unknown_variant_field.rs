//~ ERROR E0559
enum Shape { Rect { w: u32, h: u32 } }
fn main() { let _ = Shape::Rect { w: 1, d: 2 }; }
