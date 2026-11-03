//~ ERROR E0506
fn main() {
    let mut x = 5;
    let r = &x;
    x = 6;
    println!("{} {}", r, x);
}
