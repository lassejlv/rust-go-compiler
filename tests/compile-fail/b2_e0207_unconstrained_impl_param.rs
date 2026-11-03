//~ ERROR E0207
struct S;
impl<T: Default> S { fn make() -> T { T::default() } }
fn main() {}
