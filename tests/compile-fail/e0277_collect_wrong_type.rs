//~ ERROR E0277
fn main() {
    let v: Vec<String> = vec![1, 2, 3].into_iter().collect();
    println!("{:?}", v);
}
