//~ ERROR E0599
fn main() {
    let v = vec![1, 2, 3];
    let s = v.join(",");
    println!("{}", s);
}
