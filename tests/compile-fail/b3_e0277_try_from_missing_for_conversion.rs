//~ ERROR E0277
use std::convert::TryFrom;
struct Even(u32);
fn main() {
    let e = Even::try_from(4u32);
    let _ = e;
}
