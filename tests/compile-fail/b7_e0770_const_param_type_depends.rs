//~ ERROR E0770
// The type of a const parameter cannot be another parameter.
fn f<T, const N: T>() {}
fn main() {}
