//~ ERROR E0594
fn main() {
    let mut v = vec![1];
    if let Some(x) = v.get(0) { *x = 5; }
    v.push(1);
}
