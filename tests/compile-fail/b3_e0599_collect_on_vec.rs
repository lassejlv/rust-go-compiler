//~ ERROR E0599
fn main() {
    let v = vec![1, 2];
    let s: Vec<i32> = v.collect();
    println!("{:?}", s);
}
