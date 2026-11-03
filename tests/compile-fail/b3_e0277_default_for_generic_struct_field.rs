//~ ERROR E0277
#[derive(Default)]
struct Config<T> { inner: T }
struct NoDefault;
fn main() { let c: Config<NoDefault> = Default::default(); let _ = c.inner; }
