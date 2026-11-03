//~ ERROR E0119
struct Wrapper<T>(T);
impl<T> From<Wrapper<T>> for Wrapper<T> { fn from(w: Wrapper<T>) -> Self { w } }
fn main() {}
