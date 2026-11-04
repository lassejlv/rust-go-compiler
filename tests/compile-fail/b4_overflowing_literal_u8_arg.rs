//~ ERROR literal out of range
// overflowing_literals is deny-by-default: a literal that does not fit
// its (inferred) type is an error.
fn take(x: u8) -> u8 { x }
fn main() {
    let ok: i8 = -128;
    println!("{} {}", take(255), ok);
    println!("{}", take(256));
}
