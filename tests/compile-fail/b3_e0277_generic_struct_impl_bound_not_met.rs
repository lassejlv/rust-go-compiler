//~ ERROR E0277
use std::fmt::Display;
struct Wrapper<T>(T);
impl<T: Display> Wrapper<T> { fn show(&self) -> String { format!("{}", self.0) } }
fn takes<T: Display>(_w: T) {}
struct NoDisp;
fn main() { takes(Wrapper(NoDisp)); }
