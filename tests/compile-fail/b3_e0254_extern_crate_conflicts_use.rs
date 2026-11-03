//~ ERROR E0254
extern crate std as core_alias;
use std::vec as core_alias;
fn main() {}
