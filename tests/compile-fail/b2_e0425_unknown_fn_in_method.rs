//~ ERROR E0425
struct S;
impl S { fn go(&self) { helper(); } }
fn main() { S.go(); }
