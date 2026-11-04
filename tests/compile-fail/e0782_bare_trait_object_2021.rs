//~ ERROR E0782
trait T { fn f(&self) -> i32; }
impl T for i32 { fn f(&self) -> i32 { *self } }
fn g(x: &T) -> i32 { x.f() }
fn main() { println!("{}", g(&3)); }
