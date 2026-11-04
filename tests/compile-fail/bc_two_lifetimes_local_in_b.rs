//~ ERROR E0597
#[derive(Debug, Default)]
struct Two<'a, 'b> { a: Vec<&'a str>, b: Option<&'b str> }
impl<'a, 'b> Two<'a, 'b> {
    fn set_b(&mut self, b: &'b str) { self.b = Some(b); }
    fn add_a(&mut self, a: &'a str) { self.a.push(a); }
}
fn main() {
    let mut t = Two::default();
    {
        let s = String::from("x");
        t.set_b(&s);
        t.add_a("lit");
    }
    println!("{:?}", t);
}
