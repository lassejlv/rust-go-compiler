//~ ERROR E0603
mod a {
    struct Secret;
    pub mod b { pub fn f() {} }
}
use a::*;
fn main() { let _ = a::Secret; b::f(); }
