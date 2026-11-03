//~ ERROR E0407
trait A { fn a(&self); }
struct S;
impl A for S { fn a(&self) {} fn b(&self) {} }
fn main() {}
