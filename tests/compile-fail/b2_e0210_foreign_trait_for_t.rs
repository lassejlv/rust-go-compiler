//~ ERROR E0210
use std::fmt::Display;
struct Mine;
impl<T> Display for T where T: Clone {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { write!(f, "x") }
}
fn main() {}
