//~ ERROR E0276
// The impl method's bound is not required by the trait.
trait Store {
    fn put<X>(&mut self, x: X);
}
struct S(usize);
impl Store for S {
    fn put<X: Copy>(&mut self, _x: X) {
        self.0 += 1;
    }
}
fn main() {
    let mut s = S(0);
    s.put(1);
}
