//~ ERROR E0716
fn make() -> Vec<i32> { vec![1, 2] }
fn main() {
    let s: &[i32];
    s = &make()[..];
    println!("{:?}", s);
}
