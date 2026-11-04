//~ ERROR E0221
trait Source { type Item; }
trait Sink { type Item; }
fn pump<T: Source + Sink>(x: T::Item) {}
fn main() {}
