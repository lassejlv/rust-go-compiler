//~ ERROR E0015
// A generic const fn cannot call a trait method (checked without instances).
trait Zero {
    fn zero() -> Self;
}
const fn make<T: Zero>() -> T {
    T::zero()
}
fn main() {}
