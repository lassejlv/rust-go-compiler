//~ ERROR E0369
enum C { R, G }
fn main() { if C::R == C::G { println!("eq"); } }
