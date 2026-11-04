//~ ERROR lint
// dangerous_implicit_autorefs: a range index is an overloaded `Index`
// call that borrows `*ptr`.
fn view<'a>(ptr: *const [u8]) -> &'a [u8] {
    unsafe { &(*ptr)[..] }
}

fn main() {
    let b = [7u8, 8, 9];
    println!("{:?}", view(&b[..]));
}
