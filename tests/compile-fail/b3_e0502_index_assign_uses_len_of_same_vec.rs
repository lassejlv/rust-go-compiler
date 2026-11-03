//~ ERROR E0502
fn main() {
    let mut v = vec![0; 3];
    v[v.len() - 1] = 5;
    println!("{:?}", v);
}
