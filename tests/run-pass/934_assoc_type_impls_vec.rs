use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;

trait Collection {
    type Elem;
    fn empty() -> Self;
    fn add(&mut self, e: Self::Elem);
    fn size(&self) -> usize;
    fn elems(&self) -> Vec<&Self::Elem>;
}

impl<T> Collection for Vec<T> {
    type Elem = T;
    fn empty() -> Self { Vec::new() }
    fn add(&mut self, e: T) { self.push(e) }
    fn size(&self) -> usize { self.len() }
    fn elems(&self) -> Vec<&T> { self.iter().collect() }
}
impl<T> Collection for VecDeque<T> {
    type Elem = T;
    fn empty() -> Self { VecDeque::new() }
    fn add(&mut self, e: T) { self.push_front(e) }
    fn size(&self) -> usize { self.len() }
    fn elems(&self) -> Vec<&T> { self.iter().collect() }
}
impl<K: Ord, V> Collection for BTreeMap<K, V> {
    type Elem = (K, V);
    fn empty() -> Self { BTreeMap::new() }
    fn add(&mut self, (k, v): (K, V)) { self.insert(k, v); }
    fn size(&self) -> usize { self.len() }
    fn elems(&self) -> Vec<&(K, V)> { Vec::new() }
}
impl Collection for String {
    type Elem = char;
    fn empty() -> Self { String::new() }
    fn add(&mut self, c: char) { self.push(c) }
    fn size(&self) -> usize { self.chars().count() }
    fn elems(&self) -> Vec<&char> { vec![] }
}

fn fill<C: Collection>(items: impl IntoIterator<Item = C::Elem>) -> C {
    let mut c = C::empty();
    for i in items { c.add(i); }
    c
}

fn transfer<A, B>(a: &A) -> B
where
    A: Collection,
    B: Collection<Elem = A::Elem>,
    A::Elem: Clone,
{
    let mut b = B::empty();
    for e in a.elems() { b.add(e.clone()); }
    b
}

fn describe<C: Collection + Debug>(c: &C) -> String where C::Elem: Debug {
    format!("{:?} size={} first={:?}", c, c.size(), c.elems().first())
}

fn main() {
    let v: Vec<i32> = fill(vec![1, 2, 3]);
    let d: VecDeque<i32> = fill(vec![1, 2, 3]);
    let m: BTreeMap<&str, u8> = fill(vec![("b", 2), ("a", 1), ("b", 3)]);
    let s: String = fill("hello".chars().rev());
    println!("{}", describe(&v));
    println!("{}", describe(&d));
    println!("{:?} {}", m, m.size());
    println!("{} {}", s, s.size());
    let d2: VecDeque<i32> = transfer(&v);
    let v2: Vec<i32> = transfer(&d);
    println!("{:?} {:?}", d2, v2);
    let words: Vec<String> = fill(["x".to_string(), "yy".to_string()]);
    let wd: VecDeque<String> = transfer(&words);
    println!("{}", describe(&wd));
}
