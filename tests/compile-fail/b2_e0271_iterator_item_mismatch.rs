//~ ERROR E0271
fn sum_all<I: Iterator<Item = i32>>(i: I) -> i32 { i.sum() }
fn main() { sum_all(vec![1u8, 2].into_iter()); }
