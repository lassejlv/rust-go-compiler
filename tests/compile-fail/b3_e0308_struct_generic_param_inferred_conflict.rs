//~ ERROR E0308
struct Pair<T> { a: T, b: T }
fn main() { let p = Pair { a: 1, b: 2.0 }; let _ = (p.a, p.b); }
