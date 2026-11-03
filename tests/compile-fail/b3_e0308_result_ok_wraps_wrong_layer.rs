//~ ERROR E0308
fn parse(s: &str) -> Result<Option<i32>, String> {
    match s.parse::<i32>() {
        Ok(n) => Ok(n),
        Err(e) => Err(e.to_string()),
    }
}
fn main() { let _ = parse("1"); }
