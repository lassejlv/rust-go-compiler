//~ ERROR E0220
trait Store { type Key; type Value; }
fn lookup<S: Store<Key = u32, Val = String>>(s: &S) {}
fn main() {}
