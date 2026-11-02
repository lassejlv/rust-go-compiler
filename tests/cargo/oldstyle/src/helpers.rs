use std::iter::Iterator;
pub fn twice(x: i32) -> i32 { ::helpers::times(x, 2) }
pub fn times(x: i32, n: i32) -> i32 { x * n }
pub fn sum(xs: &[i64]) -> i64 { xs.iter().fold(0, |a, b| a + b) }
