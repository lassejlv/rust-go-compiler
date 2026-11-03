//~ ERROR E0080
const fn check(n: u32) -> u32 { if n > 10 { panic!("too big") } n }
const N: u32 = check(20);
fn main() { println!("{}", N); }
