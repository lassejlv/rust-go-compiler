//~ ERROR E0425
struct S;
impl S { fn f(&self) -> Widget { todo!() } }
fn main() {}
