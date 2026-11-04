//~ ERROR lint
// dangerous_implicit_autorefs: `[T]::len` called on the dereference of a
// raw slice pointer creates a reference (use `ptr.len()` instead).
fn count(raw: *const [i32]) -> usize {
    unsafe { (*raw).len() }
}

fn main() {
    let a = [1, 2, 3];
    println!("{}", count(&a[..]));
}
