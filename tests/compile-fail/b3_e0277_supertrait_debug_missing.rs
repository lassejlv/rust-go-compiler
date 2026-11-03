//~ ERROR E0277
use std::fmt::Debug;
trait Loggable: Debug { fn log(&self) { println!("{:?}", self); } }
struct Item;
impl Loggable for Item {}
fn main() { Item.log(); }
