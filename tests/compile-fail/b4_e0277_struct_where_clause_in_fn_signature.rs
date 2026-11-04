//~ ERROR E0277
trait Key {}
impl Key for String {}
struct Index<K: Key> { keys: Vec<K> }
fn size(i: &Index<f32>) -> usize { i.keys.len() }
fn main() {}
