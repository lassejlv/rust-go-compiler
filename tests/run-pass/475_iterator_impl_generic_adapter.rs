// Writing generic iterator adapters: an extension trait with default methods returning custom
// adapter structs (Interleave, Batched, Dedup-by-key, Pairwise), adapters wrapping generic I: Iterator
// with Item projections, size_hint propagation, DoubleEndedIterator forwarding, and chaining them
// with std adapters on different source iterators.
use std::fmt::Debug;

struct Interleave<A, B> {
    a: A,
    b: B,
    flag: bool,
}
impl<T, A: Iterator<Item = T>, B: Iterator<Item = T>> Iterator for Interleave<A, B> {
    type Item = T;
    fn next(&mut self) -> Option<T> {
        self.flag = !self.flag;
        if self.flag {
            self.a.next().or_else(|| self.b.next())
        } else {
            self.b.next().or_else(|| self.a.next())
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let (al, ah) = self.a.size_hint();
        let (bl, bh) = self.b.size_hint();
        (al + bl, ah.zip(bh).map(|(x, y)| x + y))
    }
}

struct Batched<I: Iterator> {
    it: I,
    n: usize,
}
impl<I: Iterator> Iterator for Batched<I> {
    type Item = Vec<I::Item>;
    fn next(&mut self) -> Option<Vec<I::Item>> {
        let v: Vec<I::Item> = self.it.by_ref().take(self.n).collect();
        if v.is_empty() { None } else { Some(v) }
    }
}

struct DedupBy<I: Iterator, K, F> {
    it: std::iter::Peekable<I>,
    key: F,
    _k: std::marker::PhantomData<K>,
}
impl<I: Iterator, K: PartialEq, F: FnMut(&I::Item) -> K> Iterator for DedupBy<I, K, F> {
    type Item = (I::Item, usize);
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.it.next()?;
        let k = (self.key)(&first);
        let mut count = 1;
        while let Some(nx) = self.it.peek() {
            if (self.key)(nx) == k {
                self.it.next();
                count += 1;
            } else {
                break;
            }
        }
        Some((first, count))
    }
}

struct Pairwise<I: Iterator> {
    it: I,
    prev: Option<I::Item>,
}
impl<I> Iterator for Pairwise<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        if self.prev.is_none() {
            self.prev = self.it.next();
        }
        let cur = self.it.next()?;
        let p = self.prev.replace(cur.clone())?;
        Some((p, cur))
    }
}

struct Mapped<I, F> {
    it: I,
    f: F,
}
impl<B, I: Iterator, F: FnMut(I::Item) -> B> Iterator for Mapped<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        self.it.next().map(&mut self.f)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        self.it.size_hint()
    }
}
impl<B, I: DoubleEndedIterator, F: FnMut(I::Item) -> B> DoubleEndedIterator for Mapped<I, F> {
    fn next_back(&mut self) -> Option<B> {
        self.it.next_back().map(&mut self.f)
    }
}

trait IterExt: Iterator + Sized {
    fn interleave<B: Iterator<Item = Self::Item>>(self, b: B) -> Interleave<Self, B> {
        Interleave { a: self, b, flag: false }
    }
    fn batched(self, n: usize) -> Batched<Self> {
        Batched { it: self, n }
    }
    fn dedup_by_key_count<K: PartialEq, F: FnMut(&Self::Item) -> K>(self, key: F) -> DedupBy<Self, K, F> {
        DedupBy { it: self.peekable(), key, _k: std::marker::PhantomData }
    }
    fn pairwise(self) -> Pairwise<Self> {
        Pairwise { it: self, prev: None }
    }
    fn my_map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> Mapped<Self, F> {
        Mapped { it: self, f }
    }
    fn collect_sorted(self) -> Vec<Self::Item>
    where
        Self::Item: Ord,
    {
        let mut v: Vec<_> = self.collect();
        v.sort();
        v
    }
}
impl<I: Iterator> IterExt for I {}

fn show<T: Debug>(label: &str, it: impl Iterator<Item = T>) {
    println!("{}: {:?}", label, it.collect::<Vec<_>>());
}

fn main() {
    show("interleave", (1..4).interleave(10..16));
    let il = "abc".chars().interleave("XYZ".chars());
    println!("hint {:?}", il.size_hint());
    show("interleave str", il);
    show("batched", (1..=10).batched(3));
    show("batched words", "a b c d e".split(' ').batched(2).map(|b| b.join("")));
    show("dedup", [1, 1, 2, 3, 3, 3, 1].iter().dedup_by_key_count(|x| **x));
    show("dedup len", ["aa", "bb", "c", "dd", "e", "f"].into_iter().dedup_by_key_count(|s| s.len()));
    show("pairwise", [1, 4, 9, 16].iter().pairwise().map(|(a, b)| b - a));
    show("pairwise one", std::iter::once(5).pairwise());
    let m = (1..=5).my_map(|x| x * x);
    println!("hint {:?}", m.size_hint());
    show("my_map rev", m.rev());
    show("chain all", (0..20).filter(|x| x % 3 == 0).my_map(|x| x as u8 as char).interleave("abcdefgh".chars().skip(2)).batched(4).map(|v| v.into_iter().collect::<String>()));
    println!("{:?}", vec!["pear", "apple", "fig"].into_iter().my_map(str::to_uppercase).collect_sorted());
    let nested: Vec<(Vec<i32>, usize)> = (1..=12).batched(4).dedup_by_key_count(|b| b.len()).collect();
    println!("{:?}", nested);
}
