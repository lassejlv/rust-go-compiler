//~ ERROR E0276
trait Deserializer { fn next(&mut self) -> Option<u8>; }
trait Deserialize: Sized {
    fn deserialize<D: Deserializer>(d: D) -> Result<Self, String>;
}
impl Deserialize for u8 {
    // the trait does not require `D: Clone`
    fn deserialize<D: Deserializer>(mut d: D) -> Result<u8, String> where D: Clone {
        d.next().ok_or_else(|| "eof".to_string())
    }
}
fn main() {}
