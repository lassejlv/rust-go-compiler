//~ ERROR E0603
mod m { enum Hidden { A } pub fn f() { let _ = Hidden::A; } }
fn main() { let _ = m::Hidden::A; }
