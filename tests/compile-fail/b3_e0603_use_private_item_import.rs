//~ ERROR E0603
mod config { const DEFAULT: u32 = 3; pub fn get() -> u32 { DEFAULT } }
use config::DEFAULT;
fn main() { println!("{}", DEFAULT + config::get()); }
