//~ ERROR E0384
fn main() {
    let (a, mut b) = (1, 2);
    b += 1;
    a += 1;
    println!("{} {}", a, b);
}
