//~ ERROR E0119
trait Describe {}
impl<X: ?Sized> Describe for &X {}
impl Describe for &str {}
fn main() {}
