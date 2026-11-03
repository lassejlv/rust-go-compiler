//~ ERROR E0384
fn main() {
    let (a, b) = (1, 2);
    a += b;
    println!("{}", a);
}
