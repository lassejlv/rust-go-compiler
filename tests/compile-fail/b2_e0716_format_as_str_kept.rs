//~ ERROR E0716
fn main() {
    let s: &str = format!("{}-{}", 1, 2).as_str();
    println!("{}", s);
}
