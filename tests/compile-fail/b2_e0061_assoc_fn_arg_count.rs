//~ ERROR E0061
struct V;
impl V { fn new(x: i32, y: i32) -> V { let _ = (x, y); V } }
fn main() { let _ = V::new(1); }
