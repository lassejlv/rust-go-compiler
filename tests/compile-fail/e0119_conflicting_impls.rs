//~ ERROR E0119
trait Greet {
    fn hi(&self);
}

impl<T> Greet for T {
    fn hi(&self) {}
}

impl Greet for i32 {
    fn hi(&self) {}
}

fn main() {
    1.hi();
}
