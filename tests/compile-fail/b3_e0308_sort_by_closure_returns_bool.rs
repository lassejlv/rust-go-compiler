//~ ERROR E0308
fn main() {
    let mut v = vec![3, 1];
    v.sort_by(|a, b| a < b);
}
