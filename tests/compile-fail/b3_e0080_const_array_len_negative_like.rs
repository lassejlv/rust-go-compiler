//~ ERROR E0080
const N: usize = 3 - 5;
fn main() { let a = [0u8; N]; println!("{}", a.len()); }
