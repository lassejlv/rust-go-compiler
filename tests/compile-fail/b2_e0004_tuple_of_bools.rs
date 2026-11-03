//~ ERROR E0004
fn f(a: bool, b: bool) -> u8 {
    match (a, b) {
        (true, true) => 0,
        (false, _) => 1,
    }
}
fn main() { f(true, false); }
