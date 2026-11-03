//~ ERROR E0603
mod a { fn helper() {} }
mod b { pub fn go() { crate::a::helper(); } }
fn main() { b::go(); }
