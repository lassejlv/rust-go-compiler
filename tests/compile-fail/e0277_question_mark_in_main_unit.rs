//~ ERROR E0277
fn main() {
    let n: i32 = "5".parse()?;
    println!("{}", n);
}
