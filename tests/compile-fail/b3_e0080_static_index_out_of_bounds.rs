//~ ERROR E0080
static TABLE: [u8; 3] = [1, 2, 3];
const X: u8 = TABLE[5];
fn main() { println!("{}", X); }
