//~ ERROR E0658
// Negative impls are unstable.
struct S;
impl !Send for S {}
fn main() {}
