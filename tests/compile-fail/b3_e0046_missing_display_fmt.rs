//~ ERROR E0046
use std::fmt;
struct P;
impl fmt::Display for P {}
fn main() { println!("{}", P); }
