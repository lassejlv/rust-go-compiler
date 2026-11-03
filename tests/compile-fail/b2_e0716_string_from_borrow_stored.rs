//~ ERROR E0716
fn main() {
    let mut v: Vec<&str> = Vec::new();
    v.push(String::from("x").as_str());
    println!("{:?}", v);
}
