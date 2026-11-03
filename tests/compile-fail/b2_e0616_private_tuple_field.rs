//~ ERROR E0616
mod m { pub struct Id(u32); pub fn mk() -> Id { Id(1) } }
fn main() { println!("{}", m::mk().0); }
