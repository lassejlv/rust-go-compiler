//~ ERROR is forbidden as the type of a const generic parameter
// Nor can `String`.
struct Named<const N: String>;
fn main() {}
