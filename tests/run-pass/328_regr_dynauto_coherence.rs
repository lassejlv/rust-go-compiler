use std::error::Error as StdError;
#[derive(Debug)] struct E;
impl std::fmt::Display for E { fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result { f.write_str("E") } }
impl StdError for E {}
struct Mine(E);
impl From<Mine> for Box<dyn StdError + Send + Sync + 'static> { fn from(m: Mine) -> Self { Box::new(m.0) } }
impl From<Mine> for Box<dyn StdError + Send + 'static> { fn from(m: Mine) -> Self { Box::new(m.0) } }
impl From<Mine> for Box<dyn StdError + 'static> { fn from(m: Mine) -> Self { Box::new(m.0) } }
fn main() { let a: Box<dyn StdError + Send + Sync> = Mine(E).into(); let b: Box<dyn StdError> = Mine(E).into(); println!("{} {}", a, b); }
