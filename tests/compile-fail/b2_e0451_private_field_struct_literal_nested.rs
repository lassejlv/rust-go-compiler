//~ ERROR E0451
mod m { pub struct Cfg { pub a: u8, b: u8 } }
fn main() { let _ = m::Cfg { a: 1, b: 2 }; }
