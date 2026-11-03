//~ ERROR E0107
fn parse() -> Result<i32> { Ok(1) }
fn main() { let _ = parse(); }
