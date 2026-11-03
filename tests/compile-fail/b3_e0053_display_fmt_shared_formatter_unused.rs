//~ ERROR E0053
use std::fmt;
struct P;
impl fmt::Display for P {
    fn fmt(&self, _f: &fmt::Formatter) -> fmt::Result { Ok(()) }
}
fn main() { println!("{}", P); }
