//~ ERROR E0381
fn main() {
    let name: String;
    let o = Some(3);
    if let Some(n) = o { name = n.to_string(); }
    println!("{}", name);
}
