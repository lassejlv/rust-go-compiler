//~ ERROR E0499
struct Pool { items: Vec<String> }
impl Pool {
    fn first(&mut self) -> &mut String { &mut self.items[0] }
    fn last(&mut self) -> &mut String { self.items.last_mut().unwrap() }
}
fn main() {
    let mut p = Pool { items: vec!["a".into(), "b".into()] };
    let f = p.first();
    let l = p.last();
    std::mem::swap(f, l);
}
