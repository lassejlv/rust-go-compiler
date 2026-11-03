//~ ERROR E0308
fn pair<T>(a: T, b: T) -> (T, T) { (a, b) }
fn main() { pair(1u8, "x"); }
