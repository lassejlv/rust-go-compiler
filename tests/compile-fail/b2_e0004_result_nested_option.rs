//~ ERROR E0004
fn f(r: Result<Option<i32>, ()>) -> i32 {
    match r {
        Ok(Some(n)) => n,
        Err(()) => 0,
    }
}
fn main() { f(Err(())); }
