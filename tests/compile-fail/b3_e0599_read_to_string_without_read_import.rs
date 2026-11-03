//~ ERROR E0599
fn main() {
    let mut f = std::fs::File::open("x").unwrap();
    let mut s = String::new();
    f.read_to_string(&mut s).unwrap();
}
