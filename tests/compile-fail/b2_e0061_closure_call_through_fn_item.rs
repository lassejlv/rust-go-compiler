//~ ERROR E0061
fn sum3(a: i32, b: i32, c: i32) -> i32 { a + b + c }
fn main() { let f = sum3; f(1, 2); }
