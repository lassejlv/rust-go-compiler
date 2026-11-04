//~ ERROR E0091
type Id<T> = u64;
fn main() { let x: Id<String> = 5; println!("{}", x); }
