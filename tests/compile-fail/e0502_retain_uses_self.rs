//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3, 4];
    v.retain(|x| *x != v[0]);
    println!("{:?}", v);
}
