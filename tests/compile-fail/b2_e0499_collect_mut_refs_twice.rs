//~ ERROR E0499
fn main() {
    let mut v = vec![1, 2];
    let a: Vec<&mut i32> = v.iter_mut().collect();
    let b: Vec<&mut i32> = v.iter_mut().collect();
    println!("{:?} {:?}", a, b);
}
