//~ ERROR E0620
fn main() { let v = &[1u8, 2][..]; let _ = v as [u8]; }
