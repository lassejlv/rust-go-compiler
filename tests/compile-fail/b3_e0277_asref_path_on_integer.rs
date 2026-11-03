//~ ERROR E0277
use std::path::Path;
fn open<P: AsRef<Path>>(p: P) -> bool { p.as_ref().exists() }
fn main() { open(7); }
