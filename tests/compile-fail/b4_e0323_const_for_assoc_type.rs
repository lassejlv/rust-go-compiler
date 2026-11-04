//~ ERROR E0323
trait Container { type Item; }
struct Bag;
impl Container for Bag { const Item: i32 = 0; }
fn main() {}
