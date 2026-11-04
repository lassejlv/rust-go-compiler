//~ ERROR E0600
// A negative discriminant for an unsigned representation.
#[repr(u16)]
enum Code {
    Ok = 0,
    Err = -1,
}
fn main() {
    let _ = (Code::Ok, Code::Err);
}
