//~ ERROR E0277
fn twice<T: Copy>(t: T) -> (T, T) { (t, t) }
fn main() { twice(String::new()); }
