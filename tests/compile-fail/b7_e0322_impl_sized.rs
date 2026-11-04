//~ ERROR E0322
// `Sized` cannot be implemented explicitly.
struct S;
impl Sized for S {}
fn main() {}
