//~ ERROR E0449
// Visibility qualifiers are not permitted on impls.
trait Greet {
    fn hi(&self);
}
struct P;
pub impl Greet for P {
    fn hi(&self) {
        println!("hi");
    }
}
fn main() {
    P.hi();
}
