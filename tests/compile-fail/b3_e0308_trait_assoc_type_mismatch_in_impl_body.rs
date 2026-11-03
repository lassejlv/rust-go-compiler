//~ ERROR E0308
trait Source { type Out; fn get(&self) -> Self::Out; }
struct S;
impl Source for S { type Out = u32; fn get(&self) -> Self::Out { "x".to_string() } }
fn main() { S.get(); }
