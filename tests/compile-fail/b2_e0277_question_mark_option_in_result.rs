//~ ERROR E0277
fn f(v: &[i32]) -> Result<i32, String> {
    let x = v.first()?;
    Ok(*x)
}
fn main() { let _ = f(&[]); }
