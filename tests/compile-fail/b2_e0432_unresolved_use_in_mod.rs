//~ ERROR E0432
mod m { pub fn a() {} }
use m::b;
fn main() {}
