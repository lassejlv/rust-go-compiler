//~ ERROR E0716
fn main() {
    let s: &[i32];
    s = &vec![1, 2, 3][..];
    println!("{:?}", s);
}
