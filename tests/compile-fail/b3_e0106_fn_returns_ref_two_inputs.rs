//~ ERROR E0106
fn pick(a: &str, b: &str) -> &str { if a.len() > b.len() { a } else { b } }
fn main() { pick("a", "b"); }
