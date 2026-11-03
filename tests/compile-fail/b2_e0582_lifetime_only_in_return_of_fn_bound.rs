//~ ERROR E0582
fn f<F>(_: F) where F: for<'a> Fn() -> &'a i32 {}
fn main() {}
