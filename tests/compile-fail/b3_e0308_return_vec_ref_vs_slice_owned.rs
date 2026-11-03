//~ ERROR E0308
fn tail(v: &[i32]) -> Vec<i32> { &v[1..] }
fn main() { tail(&[1, 2]); }
