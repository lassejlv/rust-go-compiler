//~ ERROR E0502
fn main() {
    let mut v = vec![String::from("b"), String::from("a")];
    let names: Vec<&str> = v.iter().map(|s| s.as_str()).collect();
    v.sort();
    println!("{:?}", names);
}
