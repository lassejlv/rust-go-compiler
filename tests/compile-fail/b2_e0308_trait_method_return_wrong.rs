//~ ERROR E0308
trait Area { fn area(&self) -> f64; }
struct R { w: u32 }
impl Area for R { fn area(&self) -> f64 { self.w * 2 } }
fn main() {}
