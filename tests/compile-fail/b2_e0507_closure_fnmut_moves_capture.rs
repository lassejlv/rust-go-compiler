//~ ERROR E0507
fn apply<F: FnMut()>(mut f: F) { f(); }
fn main() {
    let s = String::from("x");
    apply(|| drop(s));
}
