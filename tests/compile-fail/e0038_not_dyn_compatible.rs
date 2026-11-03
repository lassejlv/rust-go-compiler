//~ ERROR E0038
trait Cl {
    fn dup(&self) -> Self;
}

struct S;

impl Cl for S {
    fn dup(&self) -> Self {
        S
    }
}

fn main() {
    let b: Box<dyn Cl> = Box::new(S);
}
