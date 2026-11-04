//~ ERROR E0117
// A foreign trait for a foreign type, even with a type parameter.
use std::fmt;
impl<T> fmt::Display for Vec<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vec")
    }
}
fn main() {}
