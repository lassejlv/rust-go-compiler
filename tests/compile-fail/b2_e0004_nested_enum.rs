//~ ERROR E0004
enum Inner { A, B }
enum Outer { X(Inner), Y }
fn f(o: Outer) -> u8 {
    match o {
        Outer::X(Inner::A) => 1,
        Outer::Y => 2,
    }
}
fn main() { f(Outer::Y); }
