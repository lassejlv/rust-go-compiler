//~ ERROR E0277
#[derive(PartialEq, Eq, Ord)]
struct Version(u32, u32);
fn main() {}
