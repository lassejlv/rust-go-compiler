//~ ERROR E0080
const fn pick(table: [u8; 3], i: usize) -> u8 { table[i] }
const P: u8 = pick([1, 2, 3], 7);
fn main() { println!("{}", P); }
