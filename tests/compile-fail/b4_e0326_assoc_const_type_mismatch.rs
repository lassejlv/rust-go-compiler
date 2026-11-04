//~ ERROR E0326
trait Shape { const SIDES: usize; }
struct Square;
impl Shape for Square { const SIDES: u32 = 4; }
fn main() {}
