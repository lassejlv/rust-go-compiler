//~ ERROR E0525
// Assigning a captured variable makes the closure FnMut, not Fn.
fn call<F: Fn()>(f: F) {
    f()
}
fn main() {
    let mut count = 0;
    call(|| count = 1);
    println!("{}", count);
}
