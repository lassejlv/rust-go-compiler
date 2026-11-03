//~ ERROR E0038
trait Node { fn duplicate(&self) -> Self; }
fn walk(_n: Box<dyn Node>) {}
fn main() {}
