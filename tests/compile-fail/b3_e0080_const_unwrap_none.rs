//~ ERROR E0080
const V: u32 = { let o: Option<u32> = None; o.unwrap() };
fn main() { println!("{}", V); }
