//~ ERROR E0599
fn main() {
    let v = vec![1, 2, 3];
    let n = v.iter().filter(|x| **x > 1).len();
    println!("{}", n);
}
