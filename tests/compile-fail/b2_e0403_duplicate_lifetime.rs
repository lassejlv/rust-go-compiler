//~ ERROR E0403
fn f<'a, 'a>(x: &'a str) {}
fn main() {}
