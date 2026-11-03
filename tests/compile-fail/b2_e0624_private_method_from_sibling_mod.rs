//~ ERROR E0624
mod a { pub struct S; impl S { fn hidden(&self) {} } }
mod b { pub fn call() { crate::a::S.hidden(); } }
fn main() { b::call(); }
