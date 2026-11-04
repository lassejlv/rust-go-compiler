//~ ERROR attribute cannot be used on structs
#[repr(u8)]
struct Byte { v: u8 }
fn main() { let _ = Byte { v: 1 }.v; }
