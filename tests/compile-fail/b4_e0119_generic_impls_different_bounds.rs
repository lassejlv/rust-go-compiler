//~ ERROR E0119
// Where-clauses do not make impls disjoint: a type may be both Clone and
// Copy.
trait Describe { fn describe(&self) -> String; }
impl<T: Clone> Describe for Vec<T> { fn describe(&self) -> String { "clone".into() } }
impl<T: Copy> Describe for Vec<T> { fn describe(&self) -> String { "copy".into() } }
fn main() {}
