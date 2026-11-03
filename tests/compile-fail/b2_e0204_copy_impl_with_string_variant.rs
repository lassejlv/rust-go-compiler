//~ ERROR E0204
#[derive(Clone)]
enum E { A(String), B }
impl Copy for E {}
fn main() {}
