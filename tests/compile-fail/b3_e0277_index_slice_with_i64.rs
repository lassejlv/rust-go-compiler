//~ ERROR E0277
fn main() {
    let v = [1, 2, 3];
    let i: i64 = 1;
    println!("{}", v[i]);
}
