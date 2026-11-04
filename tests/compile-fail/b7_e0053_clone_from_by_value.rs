//~ ERROR E0053
// `clone_from` takes its source by reference.
struct S(Vec<u8>);
impl Clone for S {
    fn clone(&self) -> Self {
        S(self.0.clone())
    }
    fn clone_from(&mut self, source: Self) {
        self.0 = source.0;
    }
}
fn main() {}
