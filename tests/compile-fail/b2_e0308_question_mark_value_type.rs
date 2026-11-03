//~ ERROR E0308
fn parse(s: &str) -> Result<i32, std::num::ParseIntError> {
    let n: u8 = s.parse::<i32>()?;
    Ok(n)
}
fn main() { let _ = parse("1"); }
