//~ ERROR E0277
fn takes(s: impl Into<String>) -> String { s.into() }
fn main() { takes(42); }
