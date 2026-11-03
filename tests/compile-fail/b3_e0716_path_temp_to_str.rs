//~ ERROR E0716
use std::path::PathBuf;
fn main() {
    let name: &str;
    name = PathBuf::from("/a/b").file_name().unwrap().to_str().unwrap();
    println!("{}", name);
}
