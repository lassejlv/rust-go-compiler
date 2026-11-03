//~ ERROR E0382
fn dup<T>(x: T) -> (T, T) { (x, x) }
fn main() { dup(1); }
