//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2];
    let len = || v.len();
    v.push(3);
    println!("{}", len());
}
