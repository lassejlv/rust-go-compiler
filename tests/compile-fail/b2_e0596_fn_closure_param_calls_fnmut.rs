//~ ERROR E0596
fn call_twice<F: Fn()>(f: F) { f(); f(); }
fn main() {
    let mut v = Vec::new();
    call_twice(|| v.push(1));
}
