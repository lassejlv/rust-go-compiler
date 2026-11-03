//~ ERROR E0308
fn bump(x: &mut i32) { *x += 1; }
fn main() { let mut n = 0; bump(&n); println!("{}", n); }
