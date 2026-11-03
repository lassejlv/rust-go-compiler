//~ ERROR E0432
mod util { pub fn a() {} }
use util::{a, b};
fn main() { a(); b(); }
