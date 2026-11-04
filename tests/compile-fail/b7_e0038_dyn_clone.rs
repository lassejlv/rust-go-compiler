//~ ERROR E0038
// `Clone` is not dyn compatible (it requires `Sized`).
fn dup(x: &dyn Clone) {}
fn main() {}
