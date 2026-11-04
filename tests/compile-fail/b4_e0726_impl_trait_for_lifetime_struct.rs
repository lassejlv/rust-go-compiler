//~ ERROR E0726
use std::fmt;
struct Name<'a> { s: &'a str }
impl fmt::Display for Name {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "{}", self.s) }
}
fn main() {}
