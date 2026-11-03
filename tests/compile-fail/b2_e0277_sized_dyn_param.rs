//~ ERROR E0277
trait T {}
fn f(x: dyn T) {}
fn main() {}
