//~ ERROR lint
// dangerous_implicit_autorefs: `String::len` through a raw pointer deref.
fn size(s: *const String) -> usize {
    unsafe { (*s).len() }
}

fn main() {
    let s = String::from("hello");
    println!("{}", size(&s));
}
