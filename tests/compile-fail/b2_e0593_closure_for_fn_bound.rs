//~ ERROR E0593
fn call<F: Fn(i32) -> i32>(f: F) -> i32 { f(1) }
fn main() { call(|| 3); }
