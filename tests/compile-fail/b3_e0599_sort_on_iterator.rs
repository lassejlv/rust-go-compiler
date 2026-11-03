//~ ERROR E0599
fn main() {
    let v = vec![3, 1, 2];
    let s: Vec<i32> = v.iter().cloned().sort().collect();
    println!("{:?}", s);
}
