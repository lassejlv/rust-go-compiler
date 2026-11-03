//~ ERROR E0599
fn main() {
    let f = std::fs::File::open("x").unwrap();
    for l in f.lines() { println!("{:?}", l); }
}
