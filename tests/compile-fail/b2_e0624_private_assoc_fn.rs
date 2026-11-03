//~ ERROR E0624
mod m { pub struct S; impl S { fn build() -> S { S } } }
fn main() { let _ = m::S::build(); }
