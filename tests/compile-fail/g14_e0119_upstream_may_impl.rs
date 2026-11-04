//~ ERROR E0119
use std::fmt::Display;
trait Render { fn render(&self) -> String { String::new() } }
impl<T: Display> Render for T {}
// upstream (std) may add `impl Display for Vec<u8>` in the future
impl Render for Vec<u8> {}
struct Local;
impl Render for Local {}
impl Render for Box<Local> {}
fn main() { println!("{}", Local.render()); }
