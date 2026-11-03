//~ ERROR E0599
// A trait's default method body is checked once, with `Self` unknown:
// only the trait's own items (and supertraits) are available.
trait Greet {
    fn name(&self) -> String;
    fn greet(&self) -> String {
        format!("hello {}", self.title())
    }
}

struct P;
impl Greet for P {
    fn name(&self) -> String { "p".to_string() }
}

fn main() {
    println!("{}", P.name());
}
