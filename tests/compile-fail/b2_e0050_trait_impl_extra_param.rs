//~ ERROR E0050
trait Op { fn run(&self, x: i32) -> i32; }
struct A;
impl Op for A { fn run(&self, x: i32, y: i32) -> i32 { x + y } }
fn main() {}
