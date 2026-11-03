//~ ERROR E0223
trait Tr { type Out; }
struct S;
impl Tr for S { type Out = u8; }
fn main() { let x: S::Out = 1; }
