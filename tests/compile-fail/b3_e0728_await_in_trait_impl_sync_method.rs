//~ ERROR E0728
async fn ping() {}
trait Svc { fn call(&self); }
struct S;
impl Svc for S { fn call(&self) { ping().await; } }
fn main() {}
