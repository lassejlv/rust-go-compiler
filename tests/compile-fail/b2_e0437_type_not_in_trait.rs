//~ ERROR E0437
trait T {}
struct S;
impl T for S { type Out = u8; }
fn main() {}
