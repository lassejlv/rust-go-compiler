//~ ERROR E0502
fn main() {
    let mut v = vec![1, 2, 3];
    let w: Vec<i32> = v.iter().map(|x| { v.push(*x); *x }).collect();
    println!("{:?}", w);
}
