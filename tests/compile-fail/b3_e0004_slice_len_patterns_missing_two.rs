//~ ERROR E0004
fn f(s: &[i32]) -> i32 {
    match s {
        [] => 0,
        [x] => *x,
        [a, b, c, ..] => a + b + c,
    }
}
fn main() { f(&[]); }
