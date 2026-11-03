//~ ERROR E0220
trait Shape { type Unit; }
fn f<S: Shape>(x: S::Measure) {}
fn main() {}
