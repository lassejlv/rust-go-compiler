//~ ERROR E0117
use std::fmt;

impl fmt::Display for Vec<i32> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "vec")
    }
}

fn main() {}
