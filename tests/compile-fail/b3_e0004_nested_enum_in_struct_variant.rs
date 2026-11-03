//~ ERROR E0004
enum Inner { A, B, C }
enum Outer { X { i: Inner }, Y }
fn f(o: Outer) -> u8 {
    match o {
        Outer::X { i: Inner::A } => 1,
        Outer::X { i: Inner::B } => 2,
        Outer::Y => 3,
    }
}
fn main() { f(Outer::Y); }
