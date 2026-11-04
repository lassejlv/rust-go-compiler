//~ ERROR E0539
// The reason of `#[must_use]` must be a string literal.
#[must_use = 1]
fn f() -> i32 {
    1
}
fn main() {
    let _ = f();
}
