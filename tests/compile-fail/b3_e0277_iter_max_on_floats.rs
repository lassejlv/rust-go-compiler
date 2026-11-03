//~ ERROR E0277
fn main() {
    let v = vec![1.0, 2.5];
    let m = v.iter().max();
    println!("{:?}", m);
}
