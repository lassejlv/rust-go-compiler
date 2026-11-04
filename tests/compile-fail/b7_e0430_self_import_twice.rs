//~ ERROR E0430
// `self` may appear only once in an import list.
use std::fmt::{self, self, Display};

struct S;
impl Display for S {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "S")
    }
}
fn main() {
    println!("{}", S);
}
