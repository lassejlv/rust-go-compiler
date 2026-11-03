//~ ERROR E0599
struct Point { x: i32 }
impl Point { fn origin() -> Point { Point { x: 0 } } }
fn main() { let p = Point { x: 1 }; let o = p.origin(); let _ = o.x; }
