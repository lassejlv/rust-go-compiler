//~ ERROR E0277
fn main() {
    let v = vec![1, 2, 3];
    let i: i32 = 1;
    println!("{}", v[i]);
}
