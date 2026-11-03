//~ ERROR E0005
fn main() {
    let r: Result<i32, String> = Ok(1);
    let Ok(v) = r;
    println!("{}", v);
}
