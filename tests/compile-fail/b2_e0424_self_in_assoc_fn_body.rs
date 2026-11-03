//~ ERROR E0424
struct C { n: i32 }
impl C { fn get() -> i32 { self.n } }
fn main() {}
