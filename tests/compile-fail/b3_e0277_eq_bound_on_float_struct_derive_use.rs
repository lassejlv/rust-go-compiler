//~ ERROR E0277
#[derive(PartialEq)]
struct M(f64);
fn uniq<T: Eq>(_v: &[T]) {}
fn main() { uniq(&[M(1.0)]); }
