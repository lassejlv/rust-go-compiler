//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3];
    let last = v.last().unwrap();
    v.clear();
    println!("{}", last);
}
