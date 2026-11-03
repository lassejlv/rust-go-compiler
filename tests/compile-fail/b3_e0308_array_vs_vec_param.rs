//~ ERROR E0308
fn sum(v: Vec<i32>) -> i32 { v.iter().sum() }
fn main() { sum([1, 2, 3]); }
