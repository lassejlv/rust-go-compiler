use std::iter::{FromIterator, Sum, Product};
use std::ops::Add;

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Cents(i64);
impl Add for Cents { type Output = Cents; fn add(self, o: Cents) -> Cents { Cents(self.0 + o.0) } }
impl Sum for Cents { fn sum<I: Iterator<Item = Cents>>(it: I) -> Cents { it.fold(Cents(0), |a, b| a + b) } }
impl<'a> Sum<&'a Cents> for Cents { fn sum<I: Iterator<Item = &'a Cents>>(it: I) -> Cents { it.copied().sum() } }

#[derive(Debug, Clone, Copy, PartialEq)]
struct Factor(u32);
impl Product for Factor { fn product<I: Iterator<Item = Factor>>(it: I) -> Factor { Factor(it.map(|f| f.0).product()) } }

#[derive(Debug, Default)]
struct SortedVec<T: Ord>(Vec<T>);
impl<T: Ord> FromIterator<T> for SortedVec<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self { let mut v: Vec<T> = it.into_iter().collect(); v.sort(); SortedVec(v) }
}
impl<T: Ord> Extend<T> for SortedVec<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) { for x in it { let p = self.0.binary_search(&x).unwrap_or_else(|e| e); self.0.insert(p, x); } }
}
impl<T: Ord> IntoIterator for SortedVec<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter { self.0.into_iter() }
}
impl<'a, T: Ord> IntoIterator for &'a SortedVec<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter { self.0.iter() }
}

#[derive(Debug)]
struct Histogram { buckets: [u32; 4] }
impl FromIterator<u8> for Histogram {
    fn from_iter<I: IntoIterator<Item = u8>>(it: I) -> Self {
        let mut b = [0; 4];
        for x in it { b[(x / 64) as usize] += 1; }
        Histogram { buckets: b }
    }
}
impl<'a> FromIterator<&'a str> for Histogram {
    fn from_iter<I: IntoIterator<Item = &'a str>>(it: I) -> Self { it.into_iter().flat_map(|s| s.bytes()).collect() }
}

fn collect_into<C: FromIterator<i32>>(n: i32) -> C { (1..=n).rev().collect() }

fn main() {
    let v = vec![Cents(150), Cents(275), Cents(-25)];
    let total: Cents = v.iter().sum();
    let total2: Cents = v.into_iter().filter(|c| c.0 > 0).sum();
    println!("{:?} {:?}", total, total2);
    let empty: Cents = Vec::<Cents>::new().into_iter().sum();
    println!("{:?}", empty);
    let p: Factor = (1..=5).map(Factor).product();
    println!("{:?}", p);
    let mut sv: SortedVec<i32> = vec![5, 2, 8, 1].into_iter().collect();
    println!("{:?}", sv);
    sv.extend(vec![3, 9, 0]);
    println!("{:?}", sv);
    for x in &sv { print!("{} ", x); }
    println!();
    let doubled: Vec<i32> = sv.into_iter().map(|x| x * 2).collect();
    println!("{:?}", doubled);
    let words: SortedVec<String> = "the quick brown fox".split(' ').map(String::from).collect();
    println!("{:?}", words.0);
    let h: Histogram = vec![0u8, 63, 64, 200, 255, 128].into_iter().collect();
    println!("{:?}", h);
    let h2: Histogram = vec!["AZ", "az", "~"].into_iter().collect();
    println!("{:?}", h2);
    let a: Vec<i32> = collect_into(4);
    let b: SortedVec<i32> = collect_into(4);
    let c: std::collections::VecDeque<i32> = collect_into(3);
    let d: std::collections::BTreeSet<i32> = collect_into(3);
    println!("{:?} {:?} {:?} {:?}", a, b, c, d);
    let r: Result<Vec<i32>, String> = vec!["1", "2", "x"].iter().map(|s| s.parse::<i32>().map_err(|_| format!("bad {}", s))).collect();
    let o: Option<Vec<char>> = "abc".chars().map(|c| c.to_uppercase().next()).collect();
    println!("{:?} {:?}", r, o);
    let s: String = vec!['h', 'i'].into_iter().chain("!!".chars()).collect();
    let (ev, od): (Vec<i32>, Vec<i32>) = (1..10).partition(|x| x % 2 == 0);
    println!("{} {:?} {:?}", s, ev, od);
    let (xs, ys): (Vec<i32>, Vec<char>) = vec![(1, 'a'), (2, 'b')].into_iter().unzip();
    println!("{:?} {:?}", xs, ys);
}
