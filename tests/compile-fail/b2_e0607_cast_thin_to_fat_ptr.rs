//~ ERROR E0607
fn main() { let p: *const u8 = std::ptr::null(); let _ = p as *const [u8]; }
