//~ ERROR E0119
trait It2 { fn f(&self) {} }
impl<T: Iterator> It2 for T {}
impl<U: Clone> It2 for Option<U> {}
fn main() {}
