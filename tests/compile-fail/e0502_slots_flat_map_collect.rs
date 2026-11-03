//~ ERROR E0502
fn main() {
    let mut words = vec![String::from("a b")];
    let parts: Vec<&str> = words.iter().flat_map(|s| s.split(' ')).collect();
    words.clear();
    println!("{:?}", parts);
}
