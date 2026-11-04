//~ ERROR E0276
// An extra where-clause on a method's parameter.
use std::fmt::Debug;
trait Visit {
    fn visit<T: Clone>(&self, t: T);
}
struct V;
impl Visit for V {
    fn visit<T>(&self, t: T) where T: Clone + Debug {
        println!("{:?}", t.clone());
    }
}
fn main() {
    V.visit(1);
}
