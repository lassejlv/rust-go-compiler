//~ ERROR E0573
enum E {
    A,
}

fn f() -> E::A {
    E::A
}

fn main() {
    f();
}
