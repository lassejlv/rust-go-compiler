//~ ERROR E0594
fn call<F: Fn()>(f: F) { f() }
fn main() {
    let mut n = 0;
    call(|| n += 1);
}
