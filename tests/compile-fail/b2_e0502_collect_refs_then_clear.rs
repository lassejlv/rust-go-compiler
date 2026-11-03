//~ ERROR E0502
fn main() {
    let mut v = vec![String::from("a"), String::from("b")];
    let refs: Vec<&String> = v.iter().collect();
    v.clear();
    println!("{:?}", refs);
}
