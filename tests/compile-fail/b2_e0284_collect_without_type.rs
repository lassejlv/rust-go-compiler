//~ ERROR E0284
fn main() {
    let v = "1 2".split(' ').map(|s| s.parse().unwrap()).collect();
}
