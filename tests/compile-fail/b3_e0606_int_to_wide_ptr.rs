//~ ERROR E0606
fn main() { let p = 16usize as *const str; println!("{:?}", p as *const u8); }
