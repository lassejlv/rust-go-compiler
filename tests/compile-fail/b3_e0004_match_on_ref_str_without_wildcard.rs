//~ ERROR E0004
fn f(s: &str) -> u8 { match s { "a" => 1, "b" => 2 } }
fn main() { f("a"); }
