//~ ERROR E0277
use std::collections::HashMap;
#[derive(PartialEq, Eq, Hash)]
struct Key { a: u32, w: f32 }
fn main() { let _m: HashMap<Key, u8> = HashMap::new(); }
