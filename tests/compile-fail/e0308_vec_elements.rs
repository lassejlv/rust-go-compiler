//~ ERROR E0308
fn main() {
    let v = vec![1, 2, "three"];
    println!("{}", v.len());
}
