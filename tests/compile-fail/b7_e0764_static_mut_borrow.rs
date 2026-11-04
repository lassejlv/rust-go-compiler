//~ ERROR E0764
// The same in an immutable static.
static BUF: &mut [u8] = &mut [0; 4];
fn main() {}
