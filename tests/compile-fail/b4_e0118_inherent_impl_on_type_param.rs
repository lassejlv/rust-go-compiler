//~ ERROR E0118
impl<T> T {
    fn describe(&self) -> &'static str { "something" }
}
fn main() {}
