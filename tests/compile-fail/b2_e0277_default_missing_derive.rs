//~ ERROR E0277
#[derive(Default)]
struct Cfg { inner: Inner }
struct Inner;
fn main() { let _ = Cfg::default(); }
