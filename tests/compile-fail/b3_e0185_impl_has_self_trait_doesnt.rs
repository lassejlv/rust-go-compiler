//~ ERROR E0185
trait Factory { fn build() -> Self; }
struct W;
impl Factory for W { fn build(&self) -> Self { W } }
fn main() {}
