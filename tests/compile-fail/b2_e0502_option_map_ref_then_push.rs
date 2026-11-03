//~ ERROR E0502
fn main() {
    let mut v = vec![String::from("a")];
    let first: Option<&str> = v.first().map(|s| s.as_str());
    v.push(String::from("b"));
    println!("{:?}", first);
}
