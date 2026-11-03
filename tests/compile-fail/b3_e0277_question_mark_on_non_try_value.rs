//~ ERROR E0277
fn f() -> Result<i32, String> {
    let x: i32 = 5?;
    Ok(x)
}
fn main() { let _ = f(); }
