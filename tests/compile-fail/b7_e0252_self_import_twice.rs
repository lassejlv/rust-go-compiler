//~ ERROR E0252
// `self` imported twice in one list binds `fmt` twice.
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
