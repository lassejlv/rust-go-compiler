//~ ERROR E0053
use std::fmt;
struct P;
impl fmt::Display for P {
    fn fmt(&self, f: &fmt::Formatter) -> fmt::Result { write!(f, "p") }
}
fn main() {}
