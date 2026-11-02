struct H(i32);
fn subs(b: &Box<H>) -> &[H] {
    use core::slice::from_ref;
    from_ref(b)
}
fn main() { let b = Box::new(H(4)); println!("{}", subs(&b)[0].0); }
