//~ ERROR E0618
enum E { A }
fn main() { let _ = E::A(1); }
