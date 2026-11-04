//~ ERROR defaults for generic parameters are not allowed here
// Neither can impls.
struct S<T>(T);
impl<T = u8> S<T> {
    fn get(self) -> T {
        self.0
    }
}
fn main() {
    let _ = S(1).get();
}
