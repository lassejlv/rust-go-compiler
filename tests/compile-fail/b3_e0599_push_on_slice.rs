//~ ERROR E0599
fn add(xs: &mut [i32]) { xs.push(1); }
fn main() { let mut v = vec![]; add(&mut v); }
