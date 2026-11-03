//~ ERROR E0271
fn main() {
    let mut v: Vec<String> = Vec::new();
    v.extend(vec![1, 2].into_iter());
}
