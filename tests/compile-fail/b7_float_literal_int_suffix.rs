//~ ERROR invalid suffix `u8` for float literal
// A float literal with an integer suffix.
fn main() {
    let x = 1.5u8;
    println!("{}", x);
}
