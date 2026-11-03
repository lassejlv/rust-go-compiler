//~ ERROR E0308
fn apply(f: fn(i32) -> i32, x: i32) -> i32 { f(x) }
fn double(x: i64) -> i64 { x * 2 }
fn main() { apply(double, 1); }
