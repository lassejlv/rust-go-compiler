//~ ERROR the `repr(packed)` attribute cannot be used on enums
// `repr(packed)` is for structs and unions.
#[repr(packed)]
enum E {
    A,
}
fn main() {
    let _ = E::A;
}
