//~ ERROR E0049
trait Conv { fn conv<T>(&self, t: T) -> T; }
struct C;
impl Conv for C { fn conv(&self, t: i32) -> i32 { t } }
fn main() {}
