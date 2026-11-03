//~ ERROR E0277
#[derive(Debug)]
enum AppError { Parse(std::num::ParseIntError) }
impl From<std::num::ParseIntError> for AppError {
    fn from(e: std::num::ParseIntError) -> Self { AppError::Parse(e) }
}
fn read() -> Result<String, AppError> {
    let s = std::fs::read_to_string("x")?;
    let _n: i32 = s.trim().parse()?;
    Ok(s)
}
fn main() { let _ = read(); }
