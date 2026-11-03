struct Pairs<I: Iterator> {
    inner: I,
}

impl<I: Iterator> Iterator for Pairs<I> {
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        let a = self.inner.next()?;
        let b = self.inner.next()?;
        Some((a, b))
    }
}

struct RunLength<I: Iterator>
where
    I::Item: PartialEq,
{
    inner: std::iter::Peekable<I>,
}

impl<I> Iterator for RunLength<I>
where
    I: Iterator,
    I::Item: PartialEq,
{
    type Item = (I::Item, usize);
    fn next(&mut self) -> Option<(I::Item, usize)> {
        let first = self.inner.next()?;
        let mut n = 1;
        while let Some(x) = self.inner.peek() {
            if *x == first {
                self.inner.next();
                n += 1;
            } else {
                break;
            }
        }
        Some((first, n))
    }
}

struct Scan2<I, S, F> {
    it: I,
    state: S,
    f: F,
}

impl<I, S, F, B> Iterator for Scan2<I, S, F>
where
    I: Iterator,
    S: Clone,
    F: FnMut(&S, I::Item) -> (S, B),
{
    type Item = B;
    fn next(&mut self) -> Option<B> {
        let x = self.it.next()?;
        let (ns, out) = (self.f)(&self.state, x);
        self.state = ns;
        Some(out)
    }
}

trait MyIterExt: Iterator + Sized {
    fn pairs(self) -> Pairs<Self> { Pairs { inner: self } }
    fn run_length(self) -> RunLength<Self>
    where
        Self::Item: PartialEq,
    {
        RunLength { inner: self.peekable() }
    }
    fn scan2<S: Clone, B, F: FnMut(&S, Self::Item) -> (S, B)>(self, s: S, f: F) -> Scan2<Self, S, F> {
        Scan2 { it: self, state: s, f }
    }
}
impl<I: Iterator> MyIterExt for I {}

struct Countdown(u32);
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
        if self.0 == 0 { None } else { self.0 -= 1; Some(self.0 + 1) }
    }
}

fn main() {
    let p: Vec<_> = (1..=7).pairs().collect();
    println!("{:?}", p);
    let p2: Vec<_> = "abcdef".chars().pairs().map(|(a, b)| format!("{}{}", b, a)).collect();
    println!("{:?}", p2);
    let rl: Vec<_> = "aaabccdddd".chars().run_length().collect();
    println!("{:?}", rl);
    let rl2: Vec<_> = vec![1, 1, 2, 2, 2, 1].into_iter().run_length().map(|(v, n)| v * n as i32).collect();
    println!("{:?}", rl2);
    let sc: Vec<_> = (1..=5).scan2(0, |acc, x| (acc + x, format!("{}+{}", acc, x))).collect();
    println!("{:?}", sc);
    let cd: Vec<u32> = Countdown(5).collect();
    println!("{:?}", cd);
    let cdp: Vec<_> = Countdown(6).pairs().run_length().collect();
    println!("{:?}", cdp);
    let total: u32 = Countdown(4).zip(Countdown(10)).map(|(a, b)| a * b).sum();
    println!("{}", total);
    println!("{:?}", Countdown(3).chain(Countdown(2)).collect::<Vec<_>>());
    let mut it = Countdown(3).pairs();
    println!("{:?} {:?}", it.next(), it.next());
}
