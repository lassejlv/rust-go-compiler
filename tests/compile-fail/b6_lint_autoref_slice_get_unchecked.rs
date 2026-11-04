//~ ERROR lint
// dangerous_implicit_autorefs: the slice getters are marked
// `#[rustc_no_implicit_autorefs]`.
fn third(p: *const [u64]) -> u64 {
    unsafe { *(*p).get_unchecked(2) }
}

fn main() {
    let v = [1u64, 2, 3];
    println!("{}", third(&v[..]));
}
