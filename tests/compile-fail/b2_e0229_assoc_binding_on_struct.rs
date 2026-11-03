//~ ERROR E0229
struct W<T>(T);
fn f(w: W<u8, T = u8>) {}
fn main() {}
