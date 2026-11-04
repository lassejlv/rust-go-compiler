//~ ERROR E0731
// A transparent enum has exactly one variant.
#[repr(transparent)]
enum E {
    A(u32),
    B(u32),
}
fn main() {
    let _ = (E::A(1), E::B(2));
}
