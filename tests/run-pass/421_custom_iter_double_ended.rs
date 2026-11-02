// Custom iterators implementing size_hint, DoubleEndedIterator, ExactSizeIterator, FusedIterator,
// nth/nth_back overrides, and IntoIterator for a custom collection (by value, &, &mut).
use std::iter::FusedIterator;

#[derive(Debug, Clone)]
struct Span {
    lo: i64,
    hi: i64, // exclusive
    step: i64,
}

impl Iterator for Span {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        if self.lo >= self.hi {
            return None;
        }
        let v = self.lo;
        self.lo += self.step;
        Some(v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.lo >= self.hi { 0 } else { ((self.hi - self.lo + self.step - 1) / self.step) as usize };
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<i64> {
        self.lo += self.step * n as i64;
        self.next()
    }
}

impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<i64> {
        let n = self.len();
        if n == 0 {
            return None;
        }
        let last = self.lo + (n as i64 - 1) * self.step;
        self.hi = last;
        Some(last)
    }
}
impl ExactSizeIterator for Span {}
impl FusedIterator for Span {}

struct Ring<T> {
    items: Vec<T>,
}

impl<T> IntoIterator for Ring<T> {
    type Item = T;
    type IntoIter = std::vec::IntoIter<T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.into_iter()
    }
}
impl<'a, T> IntoIterator for &'a Ring<T> {
    type Item = &'a T;
    type IntoIter = std::slice::Iter<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter()
    }
}
impl<'a, T> IntoIterator for &'a mut Ring<T> {
    type Item = &'a mut T;
    type IntoIter = std::slice::IterMut<'a, T>;
    fn into_iter(self) -> Self::IntoIter {
        self.items.iter_mut()
    }
}

impl<T> FromIterator<T> for Ring<T> {
    fn from_iter<I: IntoIterator<Item = T>>(it: I) -> Self {
        Ring { items: it.into_iter().collect() }
    }
}
impl<T> Extend<T> for Ring<T> {
    fn extend<I: IntoIterator<Item = T>>(&mut self, it: I) {
        for x in it {
            self.items.push(x);
        }
    }
}

fn span(lo: i64, hi: i64, step: i64) -> Span {
    Span { lo, hi, step }
}

fn main() {
    let s = span(0, 20, 3);
    println!("len {} hint {:?}", s.len(), s.size_hint());
    println!("fwd {:?}", s.clone().collect::<Vec<_>>());
    println!("rev {:?}", s.clone().rev().collect::<Vec<_>>());

    let mut m = s.clone();
    let a = m.next();
    let b = m.next_back();
    let c = m.next();
    let d = m.next_back();
    println!("{:?} {:?} {:?} {:?} rest {:?} len {}", a, b, c, d, m.clone().collect::<Vec<_>>(), m.len());

    let mut n = span(0, 100, 7);
    println!("nth(2) {:?} nth(0) {:?} len {}", n.nth(2), n.nth(0), n.len());
    println!("nth_back(1) {:?}", n.nth_back(1));
    println!("rposition {:?}", span(0, 30, 5).rposition(|x| x % 10 == 0));
    println!("rev skip {:?}", span(1, 10, 2).rev().skip(1).step_by(2).collect::<Vec<_>>());
    let zipped: Vec<(i64, i64)> = span(0, 5, 1).zip(span(10, 15, 1).rev()).collect();
    println!("zip rev {:?}", zipped);
    println!("empty {:?} {:?}", span(5, 5, 1).next(), span(5, 3, 1).len());
    println!("chain len {}", span(0, 10, 1).chain(span(0, 5, 1)).count());
    println!("last {:?} sum {}", span(0, 10, 4).last(), span(0, 10, 4).sum::<i64>());

    let mut ring: Ring<String> = ["x", "y"].iter().map(|s| s.to_string()).collect();
    ring.extend(vec!["z".to_string()]);
    for s in &mut ring {
        s.push('!');
    }
    for s in &ring {
        print!("{} ", s);
    }
    println!();
    let joined: Vec<String> = ring.into_iter().rev().collect();
    println!("{:?}", joined);

    let ring2: Ring<i64> = span(1, 6, 1).map(|x| x * x).collect();
    let total: i64 = (&ring2).into_iter().sum();
    println!("total {}", total);
    let enumerated: Vec<(usize, &i64)> = ring2.items.iter().enumerate().rev().take(2).collect();
    println!("{:?}", enumerated);
}
