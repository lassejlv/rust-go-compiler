// custom DoubleEndedIterator + ExactSizeIterator + FusedIterator implementations
use std::iter::FusedIterator;

#[derive(Clone)]
struct Countdown {
    lo: u32,
    hi: u32,
}
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.hi -= 1;
            Some(self.hi)
        } else {
            None
        }
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = (self.hi - self.lo) as usize;
        (n, Some(n))
    }
}
impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        if self.lo < self.hi {
            self.lo += 1;
            Some(self.lo - 1)
        } else {
            None
        }
    }
}
impl ExactSizeIterator for Countdown {}
impl FusedIterator for Countdown {}

struct Grid {
    w: usize,
    h: usize,
}
struct Cells<'a> {
    g: &'a Grid,
    front: usize,
    back: usize,
}
impl Grid {
    fn cells(&self) -> Cells<'_> {
        Cells { g: self, front: 0, back: self.w * self.h }
    }
}
impl<'a> Iterator for Cells<'a> {
    type Item = (usize, usize);
    fn next(&mut self) -> Option<(usize, usize)> {
        if self.front >= self.back {
            return None;
        }
        let i = self.front;
        self.front += 1;
        Some((i / self.g.w, i % self.g.w))
    }
    fn size_hint(&self) -> (usize, Option<usize>) {
        let n = self.back - self.front;
        (n, Some(n))
    }
    fn nth(&mut self, n: usize) -> Option<Self::Item> {
        self.front = self.front.saturating_add(n).min(self.back);
        self.next()
    }
}
impl<'a> DoubleEndedIterator for Cells<'a> {
    fn next_back(&mut self) -> Option<(usize, usize)> {
        if self.front >= self.back {
            return None;
        }
        self.back -= 1;
        Some((self.back / self.g.w, self.back % self.g.w))
    }
}
impl<'a> ExactSizeIterator for Cells<'a> {}

struct Alternate<I> {
    inner: I,
    from_front: bool,
}
impl<I: DoubleEndedIterator> Iterator for Alternate<I> {
    type Item = I::Item;
    fn next(&mut self) -> Option<I::Item> {
        let r = if self.from_front { self.inner.next() } else { self.inner.next_back() };
        self.from_front = !self.from_front;
        r
    }
}

fn main() {
    let c = Countdown { lo: 3, hi: 9 };
    println!("len={} {:?}", c.len(), c.clone().collect::<Vec<_>>());
    println!("{:?}", c.clone().rev().collect::<Vec<_>>());
    let mut c2 = c.clone();
    println!("{:?} {:?} {:?} len={}", c2.next(), c2.next_back(), c2.next(), c2.len());
    println!("{:?}", c.clone().rev().skip(2).step_by(2).collect::<Vec<_>>());
    println!("{:?}", c.clone().zip(c.clone().rev()).collect::<Vec<_>>());
    println!("{:?}", c.clone().rposition(|x| x == 7));
    println!("{:?}", c.clone().rfind(|x| x % 2 == 0));
    println!("{}", c.clone().rfold(String::new(), |acc, x| acc + &x.to_string()));
    let g = Grid { w: 3, h: 2 };
    println!("{:?}", g.cells().collect::<Vec<_>>());
    println!("{:?}", g.cells().rev().take(2).collect::<Vec<_>>());
    let mut cells = g.cells();
    println!("{:?} {:?} len={}", cells.nth(2), cells.next_back(), cells.len());
    println!("{:?}", cells.nth(10));
    println!("{:?}", g.cells().enumerate().rev().map(|(i, (r, c))| i * 100 + r * 10 + c).collect::<Vec<_>>());
    let alt = Alternate { inner: 1..=7, from_front: true };
    println!("{:?}", alt.collect::<Vec<_>>());
    let alt2 = Alternate { inner: g.cells(), from_front: false };
    println!("{:?}", alt2.take(4).collect::<Vec<_>>());
    let ch = Countdown { lo: 0, hi: 3 }.chain(Countdown { lo: 10, hi: 12 });
    println!("{:?}", ch.rev().collect::<Vec<_>>());
    let lens: Vec<usize> = (0..4).map(|k| Countdown { lo: k, hi: 5 }.len()).collect();
    println!("{:?}", lens);
    let mut e = Countdown { lo: 0, hi: 2 }.fuse();
    println!("{:?} {:?} {:?} {:?}", e.next(), e.next(), e.next(), e.next());
    let cm: Vec<_> = Countdown { lo: 0, hi: 6 }.map(|x| x * x).rev().collect();
    println!("{:?} {}", cm, Countdown { lo: 0, hi: 6 }.map(|x| x * x).len());
}
