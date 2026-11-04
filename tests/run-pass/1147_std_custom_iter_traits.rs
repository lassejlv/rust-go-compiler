// Implementing std iterator traits on custom types: Iterator with size_hint,
// DoubleEndedIterator, ExactSizeIterator, FusedIterator, FromIterator,
// Extend, Sum/Product (by value and by reference), IntoIterator for owned,
// &, and &mut, and using them with std adaptors.
use std::collections::BTreeMap;
use std::iter::{FromIterator, FusedIterator, Product, Sum};
use std::ops::{Add, Mul};

#[derive(Debug, Clone, Copy, PartialEq, Default)]
struct Money { cents: i64 }

impl Add for Money { type Output = Money; fn add(self, o: Money) -> Money { Money { cents: self.cents + o.cents } } }
impl Sum for Money { fn sum<I: Iterator<Item = Money>>(it: I) -> Money { it.fold(Money::default(), Add::add) } }
impl<'a> Sum<&'a Money> for Money { fn sum<I: Iterator<Item = &'a Money>>(it: I) -> Money { it.copied().sum() } }

#[derive(Debug, Clone, Copy, PartialEq)]
struct Frac(i64, i64);
impl Mul for Frac { type Output = Frac; fn mul(self, o: Frac) -> Frac { let (n, d) = (self.0 * o.0, self.1 * o.1); let g = gcd(n, d); Frac(n / g, d / g) } }
fn gcd(a: i64, b: i64) -> i64 { if b == 0 { a.abs().max(1) } else { gcd(b, a % b) } }
impl Product for Frac { fn product<I: Iterator<Item = Frac>>(it: I) -> Frac { it.fold(Frac(1, 1), Mul::mul) } }

/// Counts from `lo` up to `hi` (exclusive) by `step`, double-ended.
#[derive(Clone, Debug)]
struct Stepper { lo: i64, hi: i64, step: i64 }

impl Iterator for Stepper {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        if self.lo >= self.hi { return None; }
        let v = self.lo;
        self.lo += self.step;
        Some(v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.lo >= self.hi { 0 } else { ((self.hi - self.lo + self.step - 1) / self.step) as usize };
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Stepper {
    fn next_back(&mut self) -> Option<i64> {
        let n = self.len();
        if n == 0 { return None; }
        let last = self.lo + (n as i64 - 1) * self.step;
        self.hi = last;
        Some(last)
    }
}
impl ExactSizeIterator for Stepper {}
impl FusedIterator for Stepper {}

/// A bag (multiset) built on BTreeMap
#[derive(Debug, Default, PartialEq)]
struct Bag<T: Ord> { counts: BTreeMap<T, usize>, total: usize }

impl<T: Ord> Bag<T> {
    fn count(&self, t: &T) -> usize { self.counts.get(t).copied().unwrap_or(0) }
    fn iter(&self) -> BagIter<'_, T> { BagIter { inner: self.counts.iter(), cur: None } }
}
impl<T: Ord> FromIterator<T> for Bag<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self { let mut b = Bag { counts: BTreeMap::new(), total: 0 }; b.extend(it); b }
}
impl<T: Ord> Extend<T> for Bag<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for x in it { *self.counts.entry(x).or_insert(0) += 1; self.total += 1; }
    }
}
impl<'a, T: Ord + Copy + 'a> Extend<&'a T> for Bag<T> {
    fn extend<I: IntoIterator<Item = &'a T>>(&mut self, it: I) { self.extend(it.into_iter().copied()) }
}

struct BagIter<'a, T> { inner: std::collections::btree_map::Iter<'a, T, usize>, cur: Option<(&'a T, usize)> }
impl<'a, T> Iterator for BagIter<'a, T> {
    type Item = &'a T;
    fn next(&mut self) -> Option<&'a T> {
        loop {
            if let Some((t, n)) = self.cur.as_mut() {
                if *n > 0 { *n -= 1; return Some(*t); }
            }
            let (t, n) = self.inner.next()?;
            self.cur = Some((t, *n));
        }
    }
}
impl<'a, T: Ord> IntoIterator for &'a Bag<T> {
    type Item = &'a T;
    type IntoIter = BagIter<'a, T>;
    fn into_iter(self) -> BagIter<'a, T> { self.iter() }
}
impl<T: Ord + Clone> IntoIterator for Bag<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.counts.into_iter().flat_map(|(t, n)| std::iter::repeat(t).take(n)).collect::<Vec<_>>().into_iter()
    }
}

struct Grid { w: usize, cells: Vec<u8> }
impl<'a> IntoIterator for &'a mut Grid {
    type Item = &'a mut [u8];
    type IntoIter = std::slice::ChunksMut<'a, u8>;
    fn into_iter(self) -> Self::IntoIter { self.cells.chunks_mut(self.w) }
}

fn main() {
    let wallet = vec![Money { cents: 150 }, Money { cents: 275 }, Money { cents: -25 }];
    let by_ref: Money = wallet.iter().sum();
    let by_val: Money = wallet.into_iter().filter(|m| m.cents > 0).sum();
    println!("sum by ref {:?} by val {:?} empty {:?}", by_ref, by_val, Vec::<Money>::new().into_iter().sum::<Money>());
    let p: Frac = [Frac(1, 2), Frac(2, 3), Frac(3, 4)].into_iter().product();
    println!("product {:?}", p);

    let s = Stepper { lo: 0, hi: 20, step: 3 };
    println!("forward {:?} len {}", s.clone().collect::<Vec<_>>(), s.len());
    println!("rev {:?}", s.clone().rev().collect::<Vec<_>>());
    let mut both = s.clone();
    println!("mixed {:?} {:?} {:?} {:?} len {}", both.next(), both.next_back(), both.next(), both.next_back(), both.len());
    println!("rposition {:?} nth_back {:?} rfind {:?}", s.clone().rposition(|x| x % 2 == 0), s.clone().nth_back(1), s.clone().rfind(|x| x % 5 == 0));
    println!("zip rev {:?}", s.clone().zip(s.clone().rev()).take(3).collect::<Vec<_>>());
    println!("chain len {} step_by {:?} skip/take {:?}", s.clone().chain(Stepper { lo: 100, hi: 103, step: 1 }).count(), s.clone().step_by(2).collect::<Vec<_>>(), s.clone().skip(2).take(2).collect::<Vec<_>>());
    println!("rev enumerate {:?}", s.clone().enumerate().rev().take(2).collect::<Vec<_>>());
    let empty = Stepper { lo: 5, hi: 5, step: 1 };
    println!("empty len {} next {:?} back {:?}", empty.len(), empty.clone().next(), empty.clone().next_back());

    let mut bag: Bag<char> = "mississippi".chars().collect();
    println!("bag {:?} total {}", bag.counts, bag.total);
    bag.extend(['m', 'm']);
    bag.extend(&['z']);
    println!("count m {} z {} q {}", bag.count(&'m'), bag.count(&'z'), bag.count(&'q'));
    println!("iter {:?}", bag.iter().collect::<String>());
    let mut n = 0;
    for c in &bag {
        if *c == 's' { n += 1; }
    }
    println!("for over &bag s={}", n);
    let owned: Vec<char> = bag.into_iter().rev().take(4).collect();
    println!("owned into_iter rev {:?}", owned);
    let words: Bag<&str> = "a b a c b a".split(' ').collect();
    let w2: Bag<&str> = vec!["a", "a", "a", "b", "b", "c"].into_iter().collect();
    println!("bag eq {}", words == w2);

    let mut g = Grid { w: 3, cells: (0..9).collect() };
    for row in &mut g {
        row.reverse();
    }
    println!("grid rows reversed {:?}", g.cells);
    let total: u32 = (&mut g).into_iter().map(|r| r.iter().map(|&x| x as u32).sum::<u32>()).sum();
    println!("total {}", total);
    let collected: String = vec!["x", "y", "z"].into_iter().collect();
    let boxed: Box<[i32]> = (1..4).collect();
    let pair: (Vec<i32>, Vec<i32>) = (1..7).partition(|x| x % 3 == 0);
    println!("{} {:?} {:?}", collected, boxed, pair);
    let mut ext: Vec<i64> = vec![];
    ext.extend(Stepper { lo: 1, hi: 4, step: 1 }.rev());
    ext.extend_from_slice(&[9, 9]);
    println!("extend {:?}", ext);
    let m: BTreeMap<i64, char> = Stepper { lo: 0, hi: 3, step: 1 }.zip("abc".chars()).collect();
    println!("collect map {:?}", m);
    let (small, big): (Vec<i64>, Vec<i64>) = Stepper { lo: 0, hi: 10, step: 1 }.partition(|&x| x < 5);
    let unz: (Vec<i64>, String) = Stepper { lo: 0, hi: 3, step: 1 }.map(|i| (i * i, (b'a' + i as u8) as char)).unzip();
    println!("{:?} {:?} {:?}", small, big, unz);
}
