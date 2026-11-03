//~ ERROR E0277
#[derive(Clone)]
struct Wrap<T> { v: T }
struct Plain;
fn dup<T: Clone>(t: &T) -> T { t.clone() }
fn main() { dup(&Wrap { v: Plain }); }
