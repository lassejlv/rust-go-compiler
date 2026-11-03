//~ ERROR E0308
fn f(r: Result<Vec<i32>, String>) -> Vec<i32> {
    match r {
        Ok(v) => v,
        Err(e) => e,
    }
}
fn main() { f(Ok(vec![])); }
