//~ ERROR E0277
fn main() {
    let s = "Hello, World";
    let rev: Vec<&str> = s.split(", ").rev().collect();
    println!("{:?}", rev);
}
