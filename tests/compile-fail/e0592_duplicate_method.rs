//~ ERROR E0592
struct S;

impl S {
    fn f(&self) {}
}

impl S {
    fn f(&self) {}
}

fn main() {
    S.f();
}
