//~ ERROR E0053
trait Op { fn run(&self, x: i32) -> i32; }
struct A;
impl Op for A { fn run(&self, x: u32) -> i32 { x as i32 } }
fn main() {}
