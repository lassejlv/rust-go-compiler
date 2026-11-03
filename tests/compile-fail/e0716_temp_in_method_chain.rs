//~ ERROR E0716
fn main() {
    let s: &str = String::from("a b c").as_str();
    println!("{}", s);
}
