//~ ERROR E0308
fn find(v: &[i32]) -> Option<i32> { v.iter().find(|&&x| x > 1) }
fn main() { find(&[1]); }
