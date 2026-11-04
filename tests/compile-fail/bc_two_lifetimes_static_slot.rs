//~ ERROR lifetime
#[derive(Debug, Default)]
struct Two<'a, 'b> { a: Vec<&'a str>, b: Option<&'b str> }
impl<'a, 'b> Two<'a, 'b> {
    fn set_b(&mut self, b: &'b str) { self.b = Some(b); }
    fn add_a(&mut self, a: &'a str) { self.a.push(a); }
}
fn build<'s>(long: &'s str) -> Two<'static, 's> {
    let mut t = Two::default();
    t.add_a(long);
    t.set_b(long);
    t
}
fn main() { println!("{:?}", build("z")); }
