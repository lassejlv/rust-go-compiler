//~ ERROR E0432
use std::collections::Hashmap;
fn main() { let _m: Hashmap<i32, i32> = Hashmap::new(); }
