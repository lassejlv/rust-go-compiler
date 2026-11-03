//~ ERROR E0308
struct W<T> { v: T }
fn main() {
    let w: W<u8> = W { v: 300u16 };
    let _ = w.v;
}
