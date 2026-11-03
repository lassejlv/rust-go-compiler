//~ ERROR E0004
fn f(x: Option<i32>) -> i32 {
    match x {
        Some(n) if n > 0 => n,
        Some(n) if n <= 0 => -n,
        None => 0,
    }
}
fn main() { f(None); }
