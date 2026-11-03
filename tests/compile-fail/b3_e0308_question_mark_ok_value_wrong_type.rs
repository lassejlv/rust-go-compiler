//~ ERROR E0308
fn load() -> Result<u32, std::num::ParseIntError> {
    let n: u64 = "5".parse::<u32>()?;
    Ok(n)
}
fn main() { let _ = load(); }
