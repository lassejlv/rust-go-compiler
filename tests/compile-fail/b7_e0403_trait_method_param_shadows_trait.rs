//~ ERROR E0403
// A trait method's type parameter may not shadow the trait's.
trait Convert<X> {
    fn convert<X>(x: X) -> X;
}

fn main() {}
