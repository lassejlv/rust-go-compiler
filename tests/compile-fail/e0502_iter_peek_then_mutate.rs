//~ ERROR E0502
fn main() {
    let mut v = vec![3, 1, 2];
    let max = v.iter().max().unwrap();
    v.sort();
    println!("{}", max);
}
