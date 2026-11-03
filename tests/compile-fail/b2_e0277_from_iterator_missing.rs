//~ ERROR E0277
fn main() {
    let v = vec![1, 2, 3];
    let s: String = v.iter().collect();
}
