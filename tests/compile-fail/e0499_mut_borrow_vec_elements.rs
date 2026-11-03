//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2, 3];
    let a = &mut v[0];
    let b = &mut v[1];
    *a += *b;
    println!("{:?}", v);
}
