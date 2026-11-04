//~ ERROR E0463
// Also when the crate is renamed.
#[macro_use]
extern crate lazy_static_missing as ls;
fn main() {}
