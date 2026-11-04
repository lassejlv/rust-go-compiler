//~ ERROR E0600
// Unary minus is not defined for unsigned integers.
fn main() {
    let n: u32 = 5;
    let m = -n;
    println!("{}", m);
}
