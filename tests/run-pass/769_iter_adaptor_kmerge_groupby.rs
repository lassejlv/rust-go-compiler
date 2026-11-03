// Hand-written generic iterator adaptors via extension traits: k-way merge
// of sorted iterators using a BinaryHeap of Peekables, group_by on
// consecutive keys yielding (key, Vec), interleave, dedup_by_key, sliding
// pairs, and chunked batching; all generic over the input iterator.
use std::cmp::Reverse;
use std::collections::BinaryHeap;
use std::iter::Peekable;

struct KMerge<I: Iterator> where I::Item: Ord { heap: BinaryHeap<Reverse<(I::Item, usize)>>, srcs: Vec<I> }
impl<I: Iterator> Iterator for KMerge<I> where I::Item: Ord + Clone {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        let Reverse((v, i)) = self.heap.pop()?;
        if let Some(n) = self.srcs[i].next() { self.heap.push(Reverse((n, i))); }
        Some(v)
    }
}
fn kmerge<I: Iterator>(srcs: Vec<I>) -> KMerge<I> where I::Item: Ord + Clone {
    let mut srcs = srcs;
    let mut heap = BinaryHeap::new();
    for (i, s) in srcs.iter_mut().enumerate() { if let Some(v) = s.next() { heap.push(Reverse((v, i))); } }
    KMerge { heap, srcs }
}

struct GroupBy<I: Iterator, F> { it: Peekable<I>, key: F }
impl<I: Iterator, K: PartialEq, F: FnMut(&I::Item) -> K> Iterator for GroupBy<I, F> {
    type Item = (K, Vec<I::Item>);
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.it.next()?;
        let k = (self.key)(&first);
        let mut group = vec![first];
        while let Some(nx) = self.it.peek() {
            if (self.key)(nx) == k { group.push(self.it.next().unwrap()); } else { break; }
        }
        Some((k, group))
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

struct Pairs<I: Iterator> { it: I, prev: Option<I::Item> }
impl<I: Iterator> Iterator for Pairs<I> where I::Item: Clone {
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        if self.prev.is_none() { self.prev = self.it.next(); }
        let cur = self.it.next()?;
        let p = self.prev.replace(cur.clone())?;
        Some((p, cur))
    }
}

struct Batches<I> { it: I, n: usize }
impl<I: Iterator> Iterator for Batches<I> {
    type Item = Vec<I::Item>;
    fn next(&mut self) -> Option<Vec<I::Item>> {
        let b: Vec<_> = self.it.by_ref().take(self.n).collect();
        if b.is_empty() { None } else { Some(b) }
    }
}

trait IterExt: Iterator + Sized {
    fn group_by_key<K: PartialEq, F: FnMut(&Self::Item) -> K>(self, key: F) -> GroupBy<Self, F> { GroupBy { it: self.peekable(), key } }
    fn interleave<B: Iterator<Item = Self::Item>>(self, b: B) -> Interleave<Self, B> { Interleave { a: self, b, flag: false } }
    fn pairs(self) -> Pairs<Self> where Self::Item: Clone { Pairs { it: self, prev: None } }
    fn batches(self, n: usize) -> Batches<Self> { Batches { it: self, n } }
    fn dedup_by_key2<K: PartialEq, F: FnMut(&Self::Item) -> K>(self, key: F) -> std::iter::Map<GroupBy<Self, F>, fn((K, Vec<Self::Item>)) -> Self::Item> {
        self.group_by_key(key).map(|(_, mut g)| g.swap_remove(0))
    }
}
impl<I: Iterator> IterExt for I {}

fn own(s: &&str) -> String { s.to_string() }

fn main() {
    let a = vec![1, 4, 7, 10];
    let b = vec![2, 3, 8];
    let c: Vec<i32> = vec![];
    let d = vec![0, 5, 6, 11, 12];
    let merged: Vec<i32> = kmerge(vec![a.into_iter(), b.into_iter(), c.into_iter(), d.into_iter()]).collect();
    println!("kmerge {:?}", merged);
    let words = ["apple", "avocado", "banana", "blueberry", "cherry", "apricot"];
    let strs: Vec<String> = kmerge(vec![words[..3].iter().map(own as fn(&&str) -> String), words[3..].iter().map(own)]).collect();
    println!("kmerge strings {:?}", strs);

    let groups: Vec<(char, Vec<&str>)> = words.iter().copied().group_by_key(|w| w.chars().next().unwrap()).collect();
    println!("group_by {:?}", groups);
    let runs: Vec<(bool, usize)> = [1, 3, 5, 2, 4, 7, 9, 11, 6].iter().group_by_key(|x| *x % 2 == 0).map(|(k, g)| (k, g.len())).collect();
    println!("parity runs {:?}", runs);
    println!("interleave {:?}", (1..=3).interleave(10..=16).collect::<Vec<_>>());
    println!("interleave strs {:?}", "abc".chars().interleave("XY".chars()).collect::<String>());
    println!("pairs {:?}", [1, 2, 4, 8].iter().pairs().map(|(a, b)| b - a).collect::<Vec<_>>());
    println!("pairs short {:?} {:?}", [1].iter().pairs().count(), std::iter::empty::<u8>().pairs().count());
    println!("batches {:?}", (1..=10).batches(4).collect::<Vec<_>>());
    println!("dedup_by_key {:?}", ["a", "A", "b", "B", "b", "c"].iter().dedup_by_key2(|s| s.to_lowercase()).collect::<Vec<_>>());
    let chained: Vec<(usize, Vec<u32>)> = kmerge(vec![vec![1u32, 1, 2].into_iter(), vec![1, 2, 2, 3].into_iter()])
        .group_by_key(|x| *x)
        .map(|(k, g)| (g.len(), vec![k; 1]))
        .collect();
    println!("merge+group {:?}", chained);
    let total: i32 = (1..=20).batches(3).map(|b| b.iter().product::<i32>() % 1000).pairs().map(|(x, y)| (x - y).abs()).sum();
    println!("composite {}", total);
}
