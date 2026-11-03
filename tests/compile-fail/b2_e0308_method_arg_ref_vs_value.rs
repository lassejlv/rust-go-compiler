//~ ERROR E0308
struct S;
impl S { fn take(&self, s: String) -> usize { s.len() } }
fn main() { let s = String::from("a"); S.take(&s); }
