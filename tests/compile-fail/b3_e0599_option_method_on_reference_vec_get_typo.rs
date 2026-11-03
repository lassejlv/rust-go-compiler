//~ ERROR E0599
fn main() {
    let v = vec![1, 2];
    let x = v.get(0).unwrap_or_zero();
    println!("{}", x);
}
