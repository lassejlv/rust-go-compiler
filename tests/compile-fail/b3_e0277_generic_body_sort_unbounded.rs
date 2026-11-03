//~ ERROR E0277
fn sorted<T>(mut v: Vec<T>) -> Vec<T> { v.sort(); v }
fn main() { sorted(vec![3, 1]); }
