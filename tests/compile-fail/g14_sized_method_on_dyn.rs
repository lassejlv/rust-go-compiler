//~ ERROR the `put_all` method cannot be invoked on a trait object
trait Sink {
    fn put(&mut self, s: &str);
    fn put_all<I: IntoIterator>(&mut self, items: I) where Self: Sized, I::Item: std::fmt::Debug {
        for x in items { self.put(&format!("{:?}", x)); }
    }
}
// `&mut dyn Sink: Sink`, but method lookup stops at `Self = dyn Sink`
impl<S: Sink + ?Sized> Sink for &mut S { fn put(&mut self, s: &str) { (**self).put(s) } }
struct V(Vec<String>);
impl Sink for V { fn put(&mut self, s: &str) { self.0.push(s.to_string()) } }
fn feed(s: &mut dyn Sink) {
    let r: &mut dyn Sink = s;
    r.put_all(vec![1, 2]);
}
fn main() {
    let mut v = V(vec![]);
    feed(&mut v);
    println!("{:?}", v.0);
}
