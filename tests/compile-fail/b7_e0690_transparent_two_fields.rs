//~ ERROR E0690
// A transparent struct has at most one non-zero-sized field.
#[repr(transparent)]
struct Wrapper(u8, u16);
fn main() {
    let _ = Wrapper(1, 2);
}
