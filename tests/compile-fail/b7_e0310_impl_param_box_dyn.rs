//~ ERROR E0310
// Also for a type parameter of the enclosing impl.
use std::fmt::Display;
struct W<T>(T);
impl<T: Display> W<T> {
    fn into_boxed(self) -> Box<dyn Display> {
        Box::new(self.0)
    }
}
fn main() {
    println!("{}", W(3).into_boxed());
}
