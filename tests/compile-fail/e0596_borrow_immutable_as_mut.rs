//~ ERROR E0596
fn main() {
    let v = vec![1];
    v.push(2);
    println!("{:?}", v);
}
