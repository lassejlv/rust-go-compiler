//~ ERROR E0764
// A `&mut` in a constant would live for the whole program.
const SLOT: &mut i32 = &mut 0;
fn main() {}
