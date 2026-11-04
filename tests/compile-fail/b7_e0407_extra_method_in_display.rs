//~ ERROR E0407
// `Display` has only `fmt`.
use std::fmt;
struct P;
impl fmt::Display for P {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "P")
    }
    fn to_string(&self) -> String {
        String::from("P")
    }
}
fn main() {
    println!("{}", P);
}
