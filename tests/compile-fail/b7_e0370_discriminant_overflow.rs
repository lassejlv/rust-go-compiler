//~ ERROR E0370
// The implicit discriminant after 255 does not fit in a u8.
#[repr(u8)]
enum Level {
    Max = 255,
    Overflow,
}
fn main() {
    println!("{}", Level::Max as u8);
    let _ = Level::Overflow;
}
