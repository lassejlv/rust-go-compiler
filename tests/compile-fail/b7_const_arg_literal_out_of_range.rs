//~ ERROR literal out of range
// A const argument must fit its parameter's type.
fn byte<const B: u8>() -> u8 {
    B
}
fn main() {
    println!("{}", byte::<300>());
}
