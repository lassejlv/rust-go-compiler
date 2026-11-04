//~ ERROR reserved prefix
macro_rules! m { ($($t:tt)*) => { 1 } }
fn main() { let x = m!(foo"bar"); println!("{}", x); }
