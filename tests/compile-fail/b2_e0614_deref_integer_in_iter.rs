//~ ERROR E0614
fn main() {
    let v = vec![1, 2, 3];
    let s: i32 = v.iter().map(|x| **x).sum();
}
