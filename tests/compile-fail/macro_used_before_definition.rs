//~ ERROR cannot find macro
// macro_rules! macros are textually scoped: a use before the definition is
// an error.
fn main() {
    println!("{}", later!());
}
macro_rules! later { () => { 1 } }
