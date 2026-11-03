//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2];
    let mut refs: Vec<&i32> = Vec::new();
    refs.extend(v.iter());
    v.push(3);
    println!("{:?}", refs);
}
