//~ ERROR E0515
fn names() -> Vec<&'static str> {
    let owned = vec![String::from("a")];
    owned.iter().map(|s| s.as_str()).collect()
}
fn main() { println!("{:?}", names()); }
