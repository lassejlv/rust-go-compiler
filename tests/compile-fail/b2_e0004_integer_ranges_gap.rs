//~ ERROR E0004
fn f(n: u8) -> u8 {
    match n {
        0..=99 => 0,
        101..=255 => 1,
    }
}
fn main() { f(1); }
