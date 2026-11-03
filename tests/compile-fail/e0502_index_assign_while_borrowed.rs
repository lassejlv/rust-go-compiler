//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3];
    let r = &v[1];
    v[1] = 10;
    println!("{}", r);
}
