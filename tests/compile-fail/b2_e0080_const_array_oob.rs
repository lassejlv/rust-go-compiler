//~ ERROR E0080
const A: [u8; 3] = [1, 2, 3];
const B: u8 = A[5];
fn main() { println!("{}", B); }
