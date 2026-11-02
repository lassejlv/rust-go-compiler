// A growable bitset (fixedbitset-like) with set ops and an iterator over ones.
use std::fmt;
use std::ops::{BitAnd, BitOr, BitXor, Not};

#[derive(Clone, PartialEq, Eq, Default)]
struct BitSet {
    words: Vec<u64>,
    nbits: usize,
}

impl BitSet {
    fn with_capacity(nbits: usize) -> Self {
        BitSet { words: vec![0; (nbits + 63) / 64], nbits }
    }
    fn grow(&mut self, nbits: usize) {
        if nbits > self.nbits {
            self.nbits = nbits;
            self.words.resize((nbits + 63) / 64, 0);
        }
    }
    fn insert(&mut self, i: usize) -> bool {
        self.grow(i + 1);
        let (w, b) = (i / 64, i % 64);
        let was = self.words[w] & (1 << b) != 0;
        self.words[w] |= 1 << b;
        !was
    }
    fn remove(&mut self, i: usize) -> bool {
        if i >= self.nbits {
            return false;
        }
        let (w, b) = (i / 64, i % 64);
        let was = self.words[w] & (1 << b) != 0;
        self.words[w] &= !(1u64 << b);
        was
    }
    fn contains(&self, i: usize) -> bool {
        i < self.nbits && self.words[i / 64] >> (i % 64) & 1 == 1
    }
    fn count_ones(&self) -> usize {
        self.words.iter().map(|w| w.count_ones() as usize).sum()
    }
    fn toggle_range(&mut self, lo: usize, hi: usize) {
        self.grow(hi);
        for i in lo..hi {
            self.words[i / 64] ^= 1 << (i % 64);
        }
    }
    fn ones(&self) -> Ones<'_> {
        Ones { words: &self.words, idx: 0, cur: self.words.first().copied().unwrap_or(0) }
    }
    fn is_subset(&self, o: &BitSet) -> bool {
        self.ones().all(|i| o.contains(i))
    }
    fn first(&self) -> Option<usize> {
        self.ones().next()
    }
    fn last(&self) -> Option<usize> {
        for (wi, w) in self.words.iter().enumerate().rev() {
            if *w != 0 {
                return Some(wi * 64 + 63 - w.leading_zeros() as usize);
            }
        }
        None
    }
    fn zip_words(&self, o: &BitSet, f: impl Fn(u64, u64) -> u64) -> BitSet {
        let n = self.words.len().max(o.words.len());
        let mut r = BitSet::with_capacity(self.nbits.max(o.nbits));
        for i in 0..n {
            let a = self.words.get(i).copied().unwrap_or(0);
            let b = o.words.get(i).copied().unwrap_or(0);
            r.words[i] = f(a, b);
        }
        r
    }
}

struct Ones<'a> {
    words: &'a [u64],
    idx: usize,
    cur: u64,
}

impl Iterator for Ones<'_> {
    type Item = usize;
    fn next(&mut self) -> Option<usize> {
        loop {
            if self.cur != 0 {
                let tz = self.cur.trailing_zeros() as usize;
                self.cur &= self.cur - 1;
                return Some(self.idx * 64 + tz);
            }
            self.idx += 1;
            if self.idx >= self.words.len() {
                return None;
            }
            self.cur = self.words[self.idx];
        }
    }
}

impl BitOr for &BitSet {
    type Output = BitSet;
    fn bitor(self, o: &BitSet) -> BitSet {
        self.zip_words(o, |a, b| a | b)
    }
}
impl BitAnd for &BitSet {
    type Output = BitSet;
    fn bitand(self, o: &BitSet) -> BitSet {
        self.zip_words(o, |a, b| a & b)
    }
}
impl BitXor for &BitSet {
    type Output = BitSet;
    fn bitxor(self, o: &BitSet) -> BitSet {
        self.zip_words(o, |a, b| a ^ b)
    }
}
impl Not for &BitSet {
    type Output = BitSet;
    fn not(self) -> BitSet {
        let mut r = self.zip_words(self, |a, _| !a);
        let extra = r.words.len() * 64 - r.nbits;
        if extra > 0 {
            let last = r.words.len() - 1;
            r.words[last] &= u64::MAX >> extra;
        }
        r
    }
}

impl fmt::Debug for BitSet {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.debug_set().entries(self.ones()).finish()
    }
}

impl FromIterator<usize> for BitSet {
    fn from_iter<I: IntoIterator<Item = usize>>(it: I) -> Self {
        let mut b = BitSet::default();
        for i in it {
            b.insert(i);
        }
        b
    }
}

fn sieve(n: usize) -> BitSet {
    let mut composite = BitSet::with_capacity(n);
    composite.insert(0);
    composite.insert(1);
    let mut i = 2;
    while i * i < n {
        if !composite.contains(i) {
            let mut j = i * i;
            while j < n {
                composite.insert(j);
                j += i;
            }
        }
        i += 1;
    }
    !&composite
}

fn main() {
    let mut a = BitSet::with_capacity(10);
    println!("{} {} {}", a.insert(3), a.insert(3), a.insert(70));
    a.insert(64);
    a.insert(0);
    println!("{:?} count={} nbits={}", a, a.count_ones(), a.nbits);
    println!("{} {} {}", a.contains(64), a.contains(65), a.contains(1000));
    println!("remove {} {}", a.remove(64), a.remove(64));
    let b: BitSet = [1, 3, 5, 7, 70, 130].into_iter().collect();
    println!("or  {:?}", &a | &b);
    println!("and {:?}", &a & &b);
    println!("xor {:?}", &a ^ &b);
    println!("first={:?} last={:?}", b.first(), b.last());
    println!("empty first={:?} last={:?}", BitSet::default().first(), BitSet::default().last());
    let small: BitSet = [3, 70].into_iter().collect();
    println!("subset {} {}", small.is_subset(&b), b.is_subset(&small));
    let mut t = BitSet::default();
    t.toggle_range(60, 68);
    t.toggle_range(62, 64);
    println!("toggled {:?}", t);
    let primes = sieve(120);
    println!("primes<120: {} -> {:?}", primes.count_ones(), primes.ones().collect::<Vec<_>>());
    let twin: Vec<(usize, usize)> = primes.ones().filter(|&p| primes.contains(p + 2)).map(|p| (p, p + 2)).collect();
    println!("twins {:?}", twin);
}
