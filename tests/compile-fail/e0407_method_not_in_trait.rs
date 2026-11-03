//~ ERROR E0407
trait T {
    fn a(&self);
}

struct S;

impl T for S {
    fn a(&self) {}
    fn b(&self) {}
}

fn main() {
    S.a();
}
