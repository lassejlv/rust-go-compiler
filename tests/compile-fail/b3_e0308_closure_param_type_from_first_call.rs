//~ ERROR E0308
fn main() {
    let id = |x| x;
    let a = id(1);
    let b = id("s");
    println!("{} {}", a, b);
}
