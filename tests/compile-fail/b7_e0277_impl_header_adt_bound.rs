//~ ERROR E0277
// `Holder<T>` needs `T: Copy`, which the impl does not provide.
struct Holder<T: Copy>(T);
impl<T> Holder<T> {
    fn get(&self) -> usize {
        0
    }
}
fn main() {}
