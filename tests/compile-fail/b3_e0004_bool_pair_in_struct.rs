//~ ERROR E0004
struct Flags { a: bool, b: bool }
fn f(fl: Flags) -> u8 {
    match fl {
        Flags { a: true, b: true } => 3,
        Flags { a: false, .. } => 0,
    }
}
fn main() { f(Flags { a: true, b: false }); }
