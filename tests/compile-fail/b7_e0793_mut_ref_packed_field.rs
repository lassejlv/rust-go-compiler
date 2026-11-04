//~ ERROR E0793
// Also a mutable one.
#[repr(packed)]
struct P {
    a: u8,
    b: u64,
}
fn bump(x: &mut u64) {
    *x += 1;
}
fn main() {
    let mut p = P { a: 0, b: 1 };
    bump(&mut p.b);
    let _ = p.a;
}
