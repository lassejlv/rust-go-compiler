//~ ERROR E0117
use std::fmt;
impl fmt::Display for Vec<u8> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "v") }
}
fn main() {}
