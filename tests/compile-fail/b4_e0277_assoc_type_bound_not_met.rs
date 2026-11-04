//~ ERROR E0277
trait Producer { type Out: Clone; fn make(&self) -> Self::Out; }
struct Token;
struct Factory;
impl Producer for Factory {
    type Out = Token;
    fn make(&self) -> Token { Token }
}
fn main() {}
