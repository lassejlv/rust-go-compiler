//~ ERROR E0525
fn repeat<F: FnMut()>(mut f: F) { f(); f(); }
fn main() {
    let v = vec![1];
    let f = move || drop(v);
    repeat(f);
}
