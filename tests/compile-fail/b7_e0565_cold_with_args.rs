//~ ERROR E0565
// `#[cold]` takes no arguments.
#[cold(x)]
fn f() {}
fn main() {
    f();
}
