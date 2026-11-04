use std::collections::HashMap;
use std::fmt::{Debug, Display};
use std::hash::Hash;
use std::ops::{Add, Mul};

trait Tr: Iterator<Item = u32> {}
impl<I: Iterator<Item = u32>> Tr for I {}
fn first_plus<T: Tr>(mut t: T) -> u32 { let x: u32 = t.next().unwrap_or(0); x + 1 }

trait Coll {
    type Elem: Clone + Debug;
    type It: Iterator<Item = Self::Elem> + Clone;
    fn elems(&self) -> Self::It;
}
struct Evens(u32);
impl Coll for Evens {
    type Elem = u32;
    type It = std::iter::StepBy<std::ops::Range<u32>>;
    fn elems(&self) -> Self::It { (0..self.0).step_by(2) }
}
fn twice<C: Coll>(c: &C) -> Vec<C::Elem> {
    let it = c.elems();
    it.clone().chain(it).collect()
}

trait Monoid: Sized { fn empty() -> Self; fn combine(self, o: Self) -> Self; }
impl Monoid for String { fn empty() -> Self { String::new() } fn combine(mut self, o: Self) -> Self { self.push_str(&o); self } }
impl Monoid for i32 { fn empty() -> Self { 0 } fn combine(self, o: Self) -> Self { self + o } }
impl<A: Monoid, B: Monoid> Monoid for (A, B) {
    fn empty() -> Self { (A::empty(), B::empty()) }
    fn combine(self, o: Self) -> Self { (self.0.combine(o.0), self.1.combine(o.1)) }
}
impl<T: Monoid> Monoid for Option<T> {
    fn empty() -> Self { None }
    fn combine(self, o: Self) -> Self {
        match (self, o) { (Some(a), Some(b)) => Some(a.combine(b)), (a, None) => a, (None, b) => b }
    }
}
fn fold_all<M: Monoid, I: IntoIterator<Item = M>>(i: I) -> M { i.into_iter().fold(M::empty(), M::combine) }

fn group<K, V, I, F>(items: I, key: F) -> Vec<(K, Vec<V>)>
where
    I: IntoIterator<Item = V>,
    F: Fn(&V) -> K,
    K: Hash + Eq + Ord + Clone,
{
    let mut m: HashMap<K, Vec<V>> = HashMap::new();
    for it in items { m.entry(key(&it)).or_insert_with(Vec::new).push(it); }
    let mut v: Vec<_> = m.into_iter().collect();
    v.sort_by(|a, b| a.0.cmp(&b.0));
    v
}

fn poly_eval<T>(coeffs: &[T], x: T) -> T where T: Copy + Add<Output = T> + Mul<Output = T> + Default {
    coeffs.iter().rev().fold(T::default(), |acc, &c| acc * x + c)
}

fn largest_pair<'a, K: Ord + Display, V: PartialOrd>(m: &'a [(K, V)]) -> Option<&'a K> {
    let mut best: Option<&'a (K, V)> = None;
    for p in m {
        best = match best { None => Some(p), Some(b) if p.1 > b.1 => Some(p), x => x };
    }
    best.map(|(k, _)| k)
}

struct Cache<K, V, F: FnMut(&K) -> V> { f: F, m: HashMap<K, V>, misses: u32 }
impl<K: Hash + Eq + Clone, V: Clone, F: FnMut(&K) -> V> Cache<K, V, F> {
    fn new(f: F) -> Self { Cache { f, m: HashMap::new(), misses: 0 } }
    fn get(&mut self, k: K) -> V {
        if let Some(v) = self.m.get(&k) { return v.clone(); }
        self.misses += 1;
        let v = (self.f)(&k);
        self.m.insert(k, v.clone());
        v
    }
}

fn apply_n<T, F>(f: F, n: usize) -> impl Fn(T) -> T where F: Fn(T) -> T {
    move |mut x| { for _ in 0..n { x = f(x); } x }
}

fn zip_with<A, B, C>(a: impl IntoIterator<Item = A>, b: impl IntoIterator<Item = B>, f: impl Fn(A, B) -> C) -> Vec<C> {
    a.into_iter().zip(b).map(|(x, y)| f(x, y)).collect()
}

fn main() {
    println!("{}", first_plus(vec![5u32, 6].into_iter()));
    println!("{:?}", twice(&Evens(7)));
    println!("{:?}", fold_all(vec![("a".to_string(), 1), ("b".to_string(), 2)]));
    println!("{:?}", fold_all(vec![Some(3), None, Some(4)]));
    println!("{:?}", fold_all(Vec::<(i32, Option<String>)>::new()));
    println!("{:?}", group(vec!["apple", "avocado", "banana", "blueberry", "cherry"], |s| s.chars().next().unwrap()));
    println!("{:?}", group(1..=10, |n| n % 3));
    println!("{} {}", poly_eval(&[1, 2, 3], 2), poly_eval(&[0.5, 0.0, 1.0], 3.0));
    println!("{:?}", largest_pair(&[("a", 1.5), ("b", 9.0), ("c", 3.0)]));
    let mut c = Cache::new(|k: &u64| (1..=*k).product::<u64>());
    println!("{} {} {} misses={}", c.get(5), c.get(10), c.get(5), c.misses);
    let add3 = apply_n(|x: i32| x + 3, 4);
    let dbl = apply_n(|s: String| s.clone() + &s, 3);
    println!("{} {}", add3(0), dbl("ab".into()));
    println!("{:?}", zip_with(vec![1, 2, 3], ["a", "b"], |n, s| s.repeat(n)));
}
