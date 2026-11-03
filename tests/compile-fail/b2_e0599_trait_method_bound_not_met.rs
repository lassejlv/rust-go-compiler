//~ ERROR E0599
struct Wrapper<T>(T);
impl<T: Clone> Wrapper<T> { fn dup(&self) -> T { self.0.clone() } }
struct NC;
fn main() { Wrapper(NC).dup(); }
