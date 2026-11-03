//~ ERROR E0716
fn main() {
    let mut v: Vec<&String> = Vec::new();
    v.push(&format!("{}", 1));
    println!("{:?}", v);
}
