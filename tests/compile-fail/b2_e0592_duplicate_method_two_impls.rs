//~ ERROR E0592
struct S;
impl S { fn go(&self) {} }
impl S { fn go(&self) {} }
fn main() {}
