//~ ERROR E0438
trait T {}
struct S;
impl T for S { const N: u8 = 1; }
fn main() {}
