//~ ERROR E0080
const fn kib(n: u32) -> u32 { n * 1024 * 1024 * 1024 }
const BIG: u32 = kib(8);
fn main() { println!("{}", BIG); }
