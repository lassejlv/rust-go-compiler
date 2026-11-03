//~ ERROR E0746
trait T {}
struct S;
impl T for S {}
fn mk() -> dyn T { S }
fn main() {}
