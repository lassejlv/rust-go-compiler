use std::borrow::{Borrow, Cow};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::fmt::Debug;
use std::hash::Hash;
use std::iter::Sum;
use std::ops::Add;

#[derive(Debug, Default, Clone, PartialEq)]
struct Stats { n: usize, total: i64 }
impl Extend<i64> for Stats {
    fn extend<I: IntoIterator<Item = i64>>(&mut self, it: I) { for x in it { self.n += 1; self.total += x; } }
}
impl FromIterator<i64> for Stats {
    fn from_iter<I: IntoIterator<Item = i64>>(it: I) -> Self { let mut s = Stats::default(); s.extend(it); s }
}
impl<'a> Sum<&'a Stats> for Stats {
    fn sum<I: Iterator<Item = &'a Stats>>(it: I) -> Stats { it.fold(Stats::default(), |a, b| Stats { n: a.n + b.n, total: a.total + b.total }) }
}

#[derive(Debug, Default)]
struct MultiMap<K: Ord, V> { m: BTreeMap<K, Vec<V>> }
impl<K: Ord, V> Extend<(K, V)> for MultiMap<K, V> {
    fn extend<I: IntoIterator<Item = (K, V)>>(&mut self, it: I) { for (k, v) in it { self.m.entry(k).or_default().push(v); } }
}
impl<K: Ord, V> FromIterator<(K, V)> for MultiMap<K, V> {
    fn from_iter<I: IntoIterator<Item = (K, V)>>(it: I) -> Self { let mut m = MultiMap { m: BTreeMap::new() }; m.extend(it); m }
}
impl<'a, K: Ord, V> IntoIterator for &'a MultiMap<K, V> {
    type Item = (&'a K, &'a [V]);
    type IntoIter = std::iter::Map<std::collections::btree_map::Iter<'a, K, Vec<V>>, fn((&'a K, &'a Vec<V>)) -> (&'a K, &'a [V])>;
    fn into_iter(self) -> Self::IntoIter {
        fn conv<'b, K, V>((k, v): (&'b K, &'b Vec<V>)) -> (&'b K, &'b [V]) { (k, v.as_slice()) }
        self.m.iter().map(conv as fn((&'a K, &'a Vec<V>)) -> (&'a K, &'a [V]))
    }
}

#[derive(Debug, Clone, PartialEq)]
struct Config { name: String, level: u8 }
fn bump(c: &Config) -> Cow<'_, Config> {
    if c.level >= 3 { Cow::Borrowed(c) } else { let mut o = c.clone(); o.level = 3; Cow::Owned(o) }
}
fn dedupe<T: Clone + PartialEq>(xs: &[T]) -> Cow<'_, [T]> {
    if xs.windows(2).all(|w| w[0] != w[1]) { return Cow::Borrowed(xs); }
    let mut v: Vec<T> = Vec::new();
    for x in xs { if v.last() != Some(x) { v.push(x.clone()); } }
    Cow::Owned(v)
}

fn lookup<K, Q: ?Sized, V: Clone>(m: &HashMap<K, V>, q: &Q) -> Option<V> where K: Borrow<Q> + Hash + Eq, Q: Hash + Eq {
    m.get(q).cloned()
}

fn sum_opt<T: Sum<T> + Copy>(xs: &[Option<T>]) -> Option<T> { xs.iter().copied().sum() }
fn total_res<T: Add<Output = T> + Default + Debug>(xs: Vec<Result<T, String>>) -> Result<T, String> {
    xs.into_iter().try_fold(T::default(), |acc, r| Ok(acc + r?))
}

fn main() {
    let s: Stats = vec![3i64, 4, 5].into_iter().collect();
    println!("{:?}", s);
    let parsed: Result<Stats, std::num::ParseIntError> = "1 2 3".split(' ').map(|x| x.parse::<i64>()).collect();
    println!("{:?}", parsed);
    let bad: Result<Stats, _> = "1 x".split(' ').map(|x| x.parse::<i64>()).collect();
    println!("{:?}", bad.is_err());
    let opt: Option<Stats> = vec![Some(1i64), Some(2)].into_iter().collect();
    println!("{:?}", opt);
    let (evens, odds): (Stats, Vec<i64>) = (1..=6i64).partition::<Vec<_>, _>(|x| x % 2 == 0).0.into_iter().map(|x| (x, x * 10)).unzip();
    println!("{:?} {:?}", evens, odds);
    let (st, mm): (Stats, MultiMap<bool, i64>) = (1..=5i64).map(|x| (x, (x > 2, x))).unzip();
    println!("{:?} {:?}", st, mm);
    let all: Stats = [s.clone(), st.clone()].iter().sum();
    println!("{:?}", all);
    let mm2: MultiMap<&str, usize> = "a bb a ccc bb a".split(' ').map(|w| (w, w.len())).collect();
    for (k, vs) in &mm2 { println!("{} -> {:?}", k, vs); }

    let c1 = Config { name: "x".into(), level: 1 };
    let c5 = Config { name: "y".into(), level: 5 };
    for c in [&c1, &c5] {
        let b = bump(c);
        println!("{} {:?}", matches!(b, Cow::Borrowed(_)), b.level);
    }
    let owned: Config = bump(&c1).into_owned();
    println!("{:?}", owned);
    println!("{:?} {:?}", dedupe(&[1, 2, 3]), dedupe(&[1, 1, 2, 2, 2, 3, 1]));
    let mut hm: HashMap<String, i32> = HashMap::new();
    hm.insert("k".into(), 1);
    let mut hv: HashMap<Vec<u8>, &str> = HashMap::new();
    hv.insert(vec![1, 2], "v12");
    println!("{:?} {:?} {:?}", lookup(&hm, "k"), lookup(&hm, &"k".to_string()), lookup(&hv, &[1u8, 2][..]));
    let hs: HashSet<Box<str>> = ["a".into(), "b".into()].into_iter().collect();
    println!("{} {}", hs.contains("a"), hs.contains("z"));
    println!("{:?} {:?}", sum_opt(&[Some(1), Some(2)]), sum_opt(&[Some(1.5), None]));
    println!("{:?} {:?}", total_res(vec![Ok(1), Ok(2)]), total_res::<i32>(vec![Ok(1), Err("boom".into()), Ok(3)]));
}
