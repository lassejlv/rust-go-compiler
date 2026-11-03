use std::collections::BTreeMap;
use std::fmt::Debug;
use std::hash::Hash;

struct Batched<I: Iterator> { it: I, n: usize }
impl<I: Iterator> Iterator for Batched<I> {
    type Item = Vec<I::Item>;
    fn next(&mut self) -> Option<Vec<I::Item>> {
        let v: Vec<I::Item> = self.it.by_ref().take(self.n).collect();
        if v.is_empty() { None } else { Some(v) }
    }
}

struct Interleave<A, B> { a: A, b: B, flag: bool }
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Interleave<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.flag = !self.flag;
        if self.flag { self.a.next().or_else(|| self.b.next()) } else { self.b.next().or_else(|| self.a.next()) }
    }
}

struct Dedup<I: Iterator> { it: std::iter::Peekable<I> }
impl<I: Iterator> Iterator for Dedup<I> where I::Item: PartialEq {
    type Item = (I::Item, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.it.next()?;
        let mut n = 1;
        while let Some(_) = self.it.next_if(|x| *x == first) { n += 1; }
        Some((first, n))
    }
}

struct Pairwise<I: Iterator> { it: I, prev: Option<I::Item> }
impl<I: Iterator> Iterator for Pairwise<I> where I::Item: Clone {
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        if self.prev.is_none() { self.prev = Some(self.it.next()?); }
        let nx = self.it.next()?;
        let p = self.prev.replace(nx.clone())?;
        Some((p, nx))
    }
}

trait IterExt: Iterator + Sized {
    fn batched(self, n: usize) -> Batched<Self> { assert!(n > 0); Batched { it: self, n } }
    fn interleave<J: IntoIterator<Item = Self::Item>>(self, other: J) -> Interleave<Self, J::IntoIter> { Interleave { a: self, b: other.into_iter(), flag: false } }
    fn dedup_count(self) -> Dedup<Self> where Self::Item: PartialEq { Dedup { it: self.peekable() } }
    fn pairwise(self) -> Pairwise<Self> where Self::Item: Clone { Pairwise { it: self, prev: None } }
    fn counts_by<K: Ord, F: FnMut(&Self::Item) -> K>(self, mut f: F) -> BTreeMap<K, usize> {
        let mut m = BTreeMap::new();
        for x in self { *m.entry(f(&x)).or_insert(0) += 1; }
        m
    }
    fn group_into<K: Ord + Clone, F: Fn(&Self::Item) -> K>(self, f: F) -> Vec<(K, Vec<Self::Item>)> {
        let mut out: Vec<(K, Vec<Self::Item>)> = Vec::new();
        for x in self {
            let k = f(&x);
            match out.last_mut() { Some((lk, v)) if *lk == k => v.push(x), _ => out.push((k, vec![x])) }
        }
        out
    }
    fn min_max(mut self) -> Option<(Self::Item, Self::Item)> where Self::Item: PartialOrd + Clone {
        let first = self.next()?;
        Some(self.fold((first.clone(), first), |(lo, hi), x| (if x < lo { x.clone() } else { lo }, if x > hi { x } else { hi })))
    }
    fn join_with(self, sep: &str) -> String where Self::Item: std::fmt::Display {
        let mut s = String::new();
        for (i, x) in self.enumerate() { if i > 0 { s += sep; } s += &x.to_string(); }
        s
    }
}
impl<I: Iterator> IterExt for I {}

fn unique_sorted<T: Ord + Clone + Hash + Debug>(v: &[T]) -> Vec<T> { let mut u = v.to_vec(); u.sort(); u.dedup(); u }

fn main() {
    println!("{:?}", (1..=10).batched(3).collect::<Vec<_>>());
    println!("{:?}", "abcdefg".chars().batched(2).map(|v| v.into_iter().collect::<String>()).collect::<Vec<_>>());
    println!("{:?}", (1..4).interleave(vec![10, 20, 30, 40, 50]).collect::<Vec<_>>());
    println!("{:?}", "aaabccddddde".chars().dedup_count().collect::<Vec<_>>());
    println!("{:?}", [1, 1, 2, 3, 3, 3, 1].iter().dedup_count().map(|(x, n)| format!("{}x{}", n, x)).join_with(" "));
    println!("{:?}", (1..6).pairwise().map(|(a, b)| a * b).collect::<Vec<_>>());
    println!("{:?}", ["x"].iter().pairwise().count());
    let words = "one two three four five six seven eight nine ten eleven twelve";
    println!("{:?}", words.split(' ').counts_by(|w| w.len()));
    println!("{:?}", words.split(' ').counts_by(|w| w.chars().next().unwrap()));
    println!("{:?}", [1, 3, 5, 2, 4, 7, 9, 6].into_iter().group_into(|x| x % 2 == 0));
    println!("{:?}", words.split(' ').group_into(|w| w.len() > 3).into_iter().map(|(k, v)| (k, v.len())).collect::<Vec<_>>());
    println!("{:?} {:?} {:?}", [3.5, -1.0, 9.25, 0.0].iter().copied().min_max(), "hello".chars().min_max(), std::iter::empty::<u8>().min_max());
    println!("{}", (1..=5).map(|x| x * x).join_with(" + "));
    println!("{}", vec!["a", "b"].into_iter().interleave(["1", "2", "3"]).batched(2).map(|b| b.concat()).join_with("|"));
    println!("{:?}", unique_sorted(&[5, 3, 5, 1, 3, 9]));
    let nested = (0..3).map(|i| (0..i + 2).map(move |j| i * 10 + j)).flatten().batched(4).map(|b| b.iter().sum::<i32>()).collect::<Vec<_>>();
    println!("{:?}", nested);
    let chain_stats = (1..=20).filter(|n| n % 3 != 0).pairwise().filter(|(a, b)| b - a > 1).map(|(a, _)| a).min_max();
    println!("{:?}", chain_stats);
}
