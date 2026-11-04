//~ ERROR imports need to be explicitly named
// `use {self};` imports the crate root without a name.
use {self};
fn main() {}
