//~ ERROR E0451
mod geo { pub struct Meters(f64); }
fn main() { let m = geo::Meters { 0: 1.0 }; let _ = m; }
