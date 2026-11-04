//~ ERROR literal out of range
// An explicit discriminant must fit the representation type.
#[repr(u8)]
enum Byte {
    Low = 0,
    High = 256,
}
fn main() {
    let _ = (Byte::Low, Byte::High);
}
