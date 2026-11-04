//~ ERROR attribute cannot be used on structs
#[inline]
struct Point { x: i32, y: i32 }
fn main() { let p = Point { x: 1, y: 2 }; println!("{}", p.x + p.y); }
