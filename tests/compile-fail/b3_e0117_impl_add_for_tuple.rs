//~ ERROR E0117
use std::ops::Add;
impl Add for (i32, i32) { type Output = (i32, i32); fn add(self, o: Self) -> Self { (self.0 + o.0, self.1 + o.1) } }
fn main() {}
