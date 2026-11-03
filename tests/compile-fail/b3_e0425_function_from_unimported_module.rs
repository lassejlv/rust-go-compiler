//~ ERROR E0425
mod math { pub fn square(x: i32) -> i32 { x * x } }
fn main() { println!("{}", square(3)); }
