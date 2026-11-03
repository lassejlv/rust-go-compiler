//~ ERROR E0599
fn main() {
    let s = String::from("abc");
    let r: String = s.rev().collect();
    println!("{}", r);
}
