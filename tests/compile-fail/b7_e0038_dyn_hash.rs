//~ ERROR E0038
// `Hash::hash` is generic over the hasher.
use std::hash::Hash;
fn key(x: &dyn Hash) {}
fn main() {}
