//~ ERROR E0449
trait T { fn f(&self); }
struct S;
impl T for S { pub fn f(&self) {} }
fn main() {}
