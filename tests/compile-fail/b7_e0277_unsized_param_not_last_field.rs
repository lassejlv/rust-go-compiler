//~ ERROR E0277
// A `?Sized` type parameter may only be the last field.
struct Packet<T: ?Sized>(T, u8);
fn main() {}
