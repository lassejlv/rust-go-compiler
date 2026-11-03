//~ ERROR E0599
struct Id(u32);
fn main() { let s = Id(1).to_string(); println!("{}", s); }
