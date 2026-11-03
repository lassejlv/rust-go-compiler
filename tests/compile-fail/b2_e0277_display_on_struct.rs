//~ ERROR E0277
struct P { x: i32 }
fn main() { println!("{}", P { x: 1 }); }
