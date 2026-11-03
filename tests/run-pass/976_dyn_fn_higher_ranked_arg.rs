// Calls of closures declared `dyn Fn(&T) -> K` / `impl Fn(&T) -> K` (parameters,
// their fields): the argument's elided reference is higher-ranked, so
// the result cannot hold its borrow.

struct Keyer<'k, K> {
    f: Box<dyn Fn(&String) -> K + 'k>,
    g: (u8, &'k dyn Fn(&[u8]) -> &'static str),
}
impl<'k, K> Keyer<'k, K> {
    fn key(&self, s: String) -> K {
        let k = (self.f)(&s);
        drop(s);
        k
    }
    fn name(&self) -> &'static str {
        let v = vec![1u8, 2];
        (self.g.1)(&v)
    }
}
fn imp(f: impl Fn(&String) -> &'static str) -> &'static str {
    let s = String::from("x");
    f(&s)
}
fn apply(f: &dyn Fn(&String) -> &'static str) -> &'static str {
    let s = String::from("x");
    f(&s)
}
fn boxed(mut f: Box<dyn FnMut(&str) -> Option<&'static str>>) -> Option<&'static str> {
    let s = String::from("abc");
    f(&s)
}
fn main() {
    let names = ["zero", "one", "two"];
    let k = Keyer { f: Box::new(move |s: &String| names[s.len() % 3]), g: (0, &|b: &[u8]| if b.len() > 1 { "many" } else { "few" }) };
    println!("{} {}", k.key("ab".to_string()), k.name());
    println!("{}", imp(|s| if s.is_empty() { "e" } else { "n" }));
    println!("{}", apply(&|_| "st"));
    println!("{:?}", boxed(Box::new(|s| if s.len() == 3 { Some("three") } else { None })));
}
