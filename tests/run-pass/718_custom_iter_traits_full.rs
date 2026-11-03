// Custom iterators implementing DoubleEndedIterator, ExactSizeIterator,
// FusedIterator; nth/nth_back/rfold/rev/len/size_hint behavior, and a
// non-fused iterator wrapped with .fuse().
use std::iter::FusedIterator;

#[derive(Clone, Debug)]
struct Span { lo: i64, hi: i64, step: i64 } // [lo, hi) stepping

impl Iterator for Span {
    type Item = i64;
    fn next(&mut self) -> Option<i64> {
        if self.lo >= self.hi { return None; }
        let v = self.lo;
        self.lo += self.step;
        if self.lo > self.hi { self.lo = self.hi; }
        Some(v)
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = if self.lo >= self.hi { 0 } else { ((self.hi - self.lo + self.step - 1) / self.step) as usize };
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<i64> {
        self.lo = self.lo.saturating_add(self.step * n as i64).min(self.hi);
        self.next()
    }
}

impl DoubleEndedIterator for Span {
    fn next_back(&mut self) -> Option<i64> {
        let n = self.len();
        if n == 0 { return None; }
        let last = self.lo + (n as i64 - 1) * self.step;
        self.hi = last;
        Some(last)
    }
}
impl ExactSizeIterator for Span {}
impl FusedIterator for Span {}

// An iterator that yields None then Some again (not fused).
struct Flaky { n: u32 }
impl Iterator for Flaky {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        self.n += 1;
        if self.n % 3 == 0 { None } else { Some(self.n) }
    }
}

struct Grid { w: usize, h: usize }
#[derive(Clone)]
struct Cells<'a> { g: &'a Grid, front: usize, back: usize }
impl Grid { fn cells(&self) -> Cells<'_> { Cells { g: self, front: 0, back: self.w * self.h } } }
impl<'a> Iterator for Cells<'a> {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<(usize, usize)> {
        if self.front == self.back { return None; }
        let i = self.front;
        self.front += 1;
        Some((i / self.g.w, i % self.g.w))
    }
    fn size_hint(&self) -> (usize, Option<usize>) { let n = self.back - self.front; (n, Some(n)) }
}
impl<'a> DoubleEndedIterator for Cells<'a> {
    fn next_back(&mut self) -> Option<(usize, usize)> {
        if self.front == self.back { return None; }
        self.back -= 1;
        Some((self.back / self.g.w, self.back % self.g.w))
    }
}
impl<'a> ExactSizeIterator for Cells<'a> {}

fn describe<I: ExactSizeIterator + DoubleEndedIterator + Clone>(name: &str, it: I) where I::Item: std::fmt::Debug {
    let fwd: Vec<_> = it.clone().collect();
    let back: Vec<_> = it.clone().rev().collect();
    println!("{}: len={} fwd={:?} rev={:?}", name, it.len(), fwd, back);
}

fn main() {
    let s = Span { lo: 3, hi: 20, step: 4 };
    describe("span", s.clone());
    let mut m = s.clone();
    println!("front {:?} back {:?} len {} then {:?}", m.next(), m.next_back(), m.len(), m.collect::<Vec<_>>());
    let mut m = s.clone();
    println!("nth(2)={:?} nth(5)={:?} next={:?} next_back={:?}", m.nth(2), m.nth(5), m.next(), m.next_back());
    let mut m = s.clone();
    println!("nth_back(1)={:?} rest={:?}", m.nth_back(1), m.collect::<Vec<_>>());
    println!("rfold: {}", s.clone().rfold(String::new(), |acc, x| format!("{}{},", acc, x)));
    println!("rposition of 11: {:?}", s.clone().rposition(|x| x == 11));
    println!("zip rev: {:?}", s.clone().zip(s.clone().rev()).collect::<Vec<_>>());
    println!("chain len hint: {:?}", s.clone().chain(Span { lo: 0, hi: 2, step: 1 }).size_hint());
    println!("skip/take exact: {} {}", s.clone().skip(1).len(), s.clone().take(2).len());
    println!("enumerate rev: {:?}", s.clone().enumerate().rev().collect::<Vec<_>>());
    println!("last={:?} count={} min={:?} max={:?}", s.clone().last(), s.clone().count(), s.clone().min(), s.clone().max());
    let mut e = Span { lo: 5, hi: 5, step: 1 };
    println!("empty: {:?} {:?} {:?}", e.next(), e.next_back(), e.size_hint());

    let raw: Vec<Option<u32>> = { let mut f = Flaky { n: 0 }; (0..7).map(|_| f.next()).collect() };
    println!("flaky raw: {:?}", raw);
    let mut fused = Flaky { n: 0 }.fuse();
    let fz: Vec<Option<u32>> = (0..5).map(|_| fused.next()).collect();
    println!("flaky fused: {:?}", fz);
    let taken: Vec<u32> = Flaky { n: 0 }.take_while(|&x| x < 100).collect();
    println!("take_while stops at first None: {:?}", taken);

    let g = Grid { w: 3, h: 2 };
    describe("cells", g.cells());
    let mut c = g.cells();
    c.next();
    c.next_back();
    println!("cells after both ends: len={} {:?}", c.len(), c.collect::<Vec<_>>());
    let diag: Vec<_> = g.cells().filter(|(r, c)| r == c).rev().collect();
    println!("diag rev: {:?}", diag);
    let (evens, odds): (Vec<i64>, Vec<i64>) = Span { lo: 0, hi: 10, step: 1 }.rev().partition(|x| x % 2 == 0);
    println!("evens={:?} odds={:?}", evens, odds);
    let st: Vec<i64> = Span { lo: 0, hi: 30, step: 3 }.step_by(2).collect();
    println!("step_by: {:?}", st);
    let rs: Vec<i64> = Span { lo: 0, hi: 30, step: 3 }.rev().step_by(3).collect();
    println!("rev step_by: {:?}", rs);
}
