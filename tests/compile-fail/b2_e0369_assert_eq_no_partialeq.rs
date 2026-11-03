//~ ERROR E0369
#[derive(Debug)]
struct Pt(i32);
fn main() { assert_eq!(Pt(1), Pt(1)); }
