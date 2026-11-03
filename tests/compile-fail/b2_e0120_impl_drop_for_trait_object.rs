//~ ERROR E0120
trait T {}
impl Drop for dyn T { fn drop(&mut self) {} }
fn main() {}
