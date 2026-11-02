mod c { pub const A: i32 = 0; pub const B: i32 = 1; }
#[derive(Debug, Clone, Copy)]
#[repr(i16)]
pub enum K { Set = c::A as _, Cur = c::B as _ }
fn main() { println!("{:?} {}", K::Cur, K::Cur as i16); }
