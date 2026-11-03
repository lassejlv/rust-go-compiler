//~ ERROR E0004
fn f(n: i32) -> u8 {
    match n {
        i32::MIN..0 => 0,
        1..=i32::MAX => 2,
    }
}
fn main() { f(1); }
