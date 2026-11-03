//~ ERROR E0747
struct S<'a, T>(&'a T);
fn f<'a>(s: S<u8, 'a>) {}
fn main() {}
