//~ ERROR lint
// dangerous_implicit_autorefs: `*p` is a `&str` here; calling `is_empty`
// reborrows `**p` through the dereference of the raw pointer.
fn blank(p: *const &str) -> bool {
    unsafe { (*p).is_empty() }
}

fn main() {
    let s = "";
    println!("{}", blank(&s));
}
