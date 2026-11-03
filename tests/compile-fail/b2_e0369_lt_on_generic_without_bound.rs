//~ ERROR E0369
fn max<T>(a: T, b: T) -> T { if a > b { a } else { b } }
fn main() { max(1, 2); }
