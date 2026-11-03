//~ ERROR E0425
struct S { n: i32 }
impl S { fn get(&self) -> i32 { n } }
fn main() {}
