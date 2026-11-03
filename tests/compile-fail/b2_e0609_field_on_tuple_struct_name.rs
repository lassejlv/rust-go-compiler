//~ ERROR E0609
struct P(i32, i32);
fn main() { let p = P(1, 2); println!("{}", p.x); }
