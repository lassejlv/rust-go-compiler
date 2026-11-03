use std::collections::BTreeMap;
struct IterMut<'a, T> { rest: &'a mut [T] }
impl<'a, T> Iterator for IterMut<'a, T> {
    type Item = &'a mut T;
    fn next(&mut self) -> Option<&'a mut T> {
        let rest = std::mem::take(&mut self.rest);
        let (first, tail) = rest.split_first_mut()?;
        self.rest = tail;
        Some(first)
    }
}
struct Pairs<'a, T> { v: &'a mut [T] }
impl<'a, T> Pairs<'a, T> {
    fn next_pair(&mut self) -> Option<(&'a mut T, &'a mut T)> {
        let v = std::mem::take(&mut self.v);
        if v.len() < 2 { return None; }
        let (a, rest) = v.split_at_mut(1);
        let (b, rest2) = rest.split_at_mut(1);
        self.v = rest2;
        Some((&mut a[0], &mut b[0]))
    }
}
struct Counter<'m, 'k> { map: &'m mut BTreeMap<&'k str, usize>, total: usize }
impl<'m, 'k> Counter<'m, 'k> {
    fn add(&mut self, k: &'k str) { *self.map.entry(k).or_insert(0) += 1; self.total += 1; }
    fn top(&self) -> Option<(&'k str, usize)> { self.map.iter().max_by_key(|(_, &v)| v).map(|(k, v)| (*k, *v)) }
}
struct Slot<'a> { val: Option<&'a mut i32> }
impl<'a> Slot<'a> { fn bump(&mut self) -> Option<i32> { let v = self.val.as_deref_mut()?; *v += 1; Some(*v) } fn take(&mut self) -> Option<&'a mut i32> { self.val.take() } }
struct Buf<'a> { out: &'a mut String }
impl<'a> Buf<'a> { fn w(&mut self, s: &str) -> &mut Self { self.out.push_str(s); self } }
fn main() {
    let mut data = vec![1, 2, 3, 4, 5];
    for x in (IterMut { rest: &mut data }) { *x *= 10; }
    let mut p = Pairs { v: &mut data };
    while let Some((a, b)) = p.next_pair() { std::mem::swap(a, b); }
    println!("{:?}", data);
    let mut m = BTreeMap::new();
    let text = String::from("x y x z x y");
    { let mut c = Counter { map: &mut m, total: 0 }; for w in text.split(' ') { c.add(w); } println!("{:?} {}", c.top(), c.total); }
    println!("{:?}", m);
    let mut n = 5;
    let mut s = Slot { val: Some(&mut n) };
    println!("{:?} {:?}", s.bump(), s.bump());
    let r = s.take().unwrap(); *r *= 2;
    println!("{:?} {}", s.bump(), n);
    let mut out = String::new();
    Buf { out: &mut out }.w("a").w("b").w("c");
    println!("{}", out);
}
