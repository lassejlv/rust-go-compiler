//~ ERROR E0408
enum E { A(i32), B(i32), C }
fn f(e: E) -> i32 {
    match e {
        E::A(x) | E::B(y) => x,
        E::C => 0,
    }
}
fn main() { f(E::C); }
