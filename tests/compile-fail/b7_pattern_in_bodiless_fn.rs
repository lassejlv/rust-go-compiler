//~ ERROR patterns aren't allowed in functions without bodies
// `mut x` in a trait method without a body.
trait Counter {
    fn add(&mut self, mut n: u32);
}
fn main() {}
