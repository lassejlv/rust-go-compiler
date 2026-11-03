//~ ERROR E0593
fn main() {
    let v = vec![(1, 2)];
    let s: Vec<i32> = v.into_iter().map(|a, b| a + b).collect();
}
