//~ ERROR E0382
fn main() {
    let mut names: Vec<String> = Vec::new();
    let s = String::from("alice");
    names.push(s);
    println!("{} {}", names.len(), s.len());
}
