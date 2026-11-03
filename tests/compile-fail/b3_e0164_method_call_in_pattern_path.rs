//~ ERROR E0164
enum E { A }
impl E { fn new() -> E { E::A } }
fn main() { match E::A { E::new() => {} } }
