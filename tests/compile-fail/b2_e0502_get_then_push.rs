//~ ERROR E0502
fn main() {
    let mut v = vec![10, 20];
    let first = v.get(0).unwrap();
    v.push(*first);
    println!("{}", first);
}
