//~ ERROR E0599
fn main() {
    let r: Result<i32, String> = Ok(1);
    let v = r.unwrap_or_none();
    println!("{}", v);
}
