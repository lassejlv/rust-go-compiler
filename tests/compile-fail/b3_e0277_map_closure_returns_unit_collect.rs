//~ ERROR E0277
fn main() {
    let v = vec![1, 2];
    let out: Vec<i32> = v.iter().map(|x| { x * 2; }).collect();
    println!("{:?}", out);
}
