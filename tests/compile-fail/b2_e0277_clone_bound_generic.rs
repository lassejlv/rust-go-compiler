//~ ERROR E0277
fn dup<T: Clone>(t: &T) -> (T, T) { (t.clone(), t.clone()) }
struct NoClone;
fn main() { dup(&NoClone); }
