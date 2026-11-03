//~ ERROR E0502
fn main() {
    let a = vec![1];
    let mut b = vec![2];
    let z: Vec<(&i32, &i32)> = a.iter().zip(b.iter()).collect();
    b.push(3);
    println!("{:?}", z);
}
