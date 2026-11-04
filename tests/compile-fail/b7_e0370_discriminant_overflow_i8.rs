//~ ERROR E0370
// Same for a signed representation.
#[repr(i8)]
enum Small {
    A = 126,
    B,
    C,
}
fn main() {
    let _ = (Small::A, Small::B, Small::C);
}
