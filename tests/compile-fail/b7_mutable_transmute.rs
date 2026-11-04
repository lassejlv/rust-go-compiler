//~ ERROR transmuting &T to &mut T is undefined behavior
// mutable_transmutes is deny-by-default.
fn main() {
    let x = 5;
    let r: &mut i32 = unsafe { std::mem::transmute(&x) };
    *r += 1;
}
