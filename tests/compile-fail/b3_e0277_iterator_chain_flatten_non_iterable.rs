//~ ERROR E0277
fn main() {
    let v = vec![1, 2, 3];
    let f: Vec<i32> = v.into_iter().flatten().collect();
    println!("{:?}", f);
}
