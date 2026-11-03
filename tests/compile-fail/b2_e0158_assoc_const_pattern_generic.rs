//~ ERROR E0158
trait Limits {
    const MAX: u8;
    fn check(n: u8) -> bool { match n { Self::MAX => true, _ => false } }
}
fn main() {}
