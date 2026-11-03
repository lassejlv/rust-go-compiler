//~ ERROR E0046
trait Container { type Item; fn get(&self) -> Self::Item; }
struct B;
impl Container for B { fn get(&self) -> u8 { 0 } }
fn main() {}
