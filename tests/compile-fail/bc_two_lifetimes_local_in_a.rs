//~ ERROR E0597
#[derive(Debug, Default)]
struct Two<'a, 'b> { a: Vec<&'a str>, b: Option<&'b str> }
impl<'a, 'b> Two<'a, 'b> {
    fn set_b(&mut self, b: &'b str) { self.b = Some(b); }
    fn add_a(&mut self, a: &'a str) { self.a.push(a); }
    fn first_a(&self) -> Option<&'a str> { self.a.first().copied() }
}
fn keep_a<'x>(src: &'x str) -> Option<&'x str> {
    let local = String::from("temp");
    let mut t: Two<'x, '_> = Two::default();
    t.set_b(src);
    t.add_a(&local);
    t.first_a()
}
fn main() { println!("{:?}", keep_a("q")); }
