//~ ERROR E0004
fn f(a: Option<i32>, b: Result<i32, ()>) -> i32 {
    match (a, b) {
        (Some(x), Ok(y)) => x + y,
        (None, Err(())) => 0,
        (Some(x), Err(())) => x,
    }
}
fn main() { f(None, Ok(1)); }
