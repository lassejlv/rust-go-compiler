//~ ERROR E0277
use std::error::Error;
fn spawnable() -> Result<(), Box<dyn Error + Send + Sync>> {
    let e: Box<dyn Error> = "oops".into();
    Err(e)?;
    Ok(())
}
fn main() { let _ = spawnable(); }
