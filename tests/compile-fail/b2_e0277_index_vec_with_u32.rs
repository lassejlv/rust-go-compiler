//~ ERROR E0277
fn main() {
    let v = vec![1, 2];
    let i: u32 = 0;
    println!("{}", v[i]);
}
