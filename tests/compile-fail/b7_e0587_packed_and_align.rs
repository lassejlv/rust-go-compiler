//~ ERROR E0587
// `packed` and `align` conflict.
#[repr(C, packed, align(8))]
struct S(u8);
fn main() {
    let _ = S(1);
}
