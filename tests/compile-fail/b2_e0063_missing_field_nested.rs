//~ ERROR E0063
struct Inner { a: u8, b: u8 }
struct Outer { i: Inner }
fn main() { let _ = Outer { i: Inner { a: 1 } }; }
