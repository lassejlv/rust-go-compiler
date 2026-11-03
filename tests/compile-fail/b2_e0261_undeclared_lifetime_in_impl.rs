//~ ERROR E0261
struct S<'a> { r: &'a str }
impl S<'a> { fn get(&self) -> &'a str { self.r } }
fn main() {}
