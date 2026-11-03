//~ ERROR E0004
fn f(c: char) -> u8 { match c { 'a'..='z' => 1, 'A'..='Z' => 2 } }
fn main() { f('a'); }
