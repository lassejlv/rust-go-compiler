//~ ERROR E0277
// The same in a function signature.
trait Codec {
    type Encoded;
}
struct Json;
fn encode(x: i32) -> <Json as Codec>::Encoded {
    todo!()
}
fn main() {}
