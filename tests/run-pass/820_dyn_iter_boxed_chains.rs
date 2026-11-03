use std::collections::BTreeMap;
use std::fmt::Debug;

trait Source<T> {
    fn items<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a>;
    fn label(&self) -> String;
    fn count_where(&self, pred: &dyn Fn(&T) -> bool) -> usize { self.items().filter(|x| pred(x)).count() }
    fn collect_sorted(&self) -> Vec<T> where T: Ord, Self: Sized { let mut v: Vec<T> = self.items().collect(); v.sort(); v }
}

struct RangeSrc { lo: i32, hi: i32, step: usize }
struct VecSrc<T> { v: Vec<T> }
struct Fib { n: usize }
struct Mapped<S, F, T> { inner: S, f: F, _p: std::marker::PhantomData<fn(T)> }

impl Source<i32> for RangeSrc {
    fn items<'a>(&'a self) -> Box<dyn Iterator<Item = i32> + 'a> { Box::new((self.lo..self.hi).step_by(self.step)) }
    fn label(&self) -> String { format!("range({}..{} by {})", self.lo, self.hi, self.step) }
}
impl<T: Clone + Debug> Source<T> for VecSrc<T> {
    fn items<'a>(&'a self) -> Box<dyn Iterator<Item = T> + 'a> { Box::new(self.v.iter().cloned()) }
    fn label(&self) -> String { format!("vec{:?}", self.v) }
}
impl Source<u64> for Fib {
    fn items<'a>(&'a self) -> Box<dyn Iterator<Item = u64> + 'a> {
        Box::new(std::iter::successors(Some((0u64, 1u64)), |&(a, b)| Some((b, a + b))).map(|p| p.0).take(self.n))
    }
    fn label(&self) -> String { format!("fib({})", self.n) }
}
impl<T: 'static, U: 'static, S: Source<T>, F: Fn(T) -> U> Source<U> for Mapped<S, F, T> {
    fn items<'a>(&'a self) -> Box<dyn Iterator<Item = U> + 'a> { Box::new(self.inner.items().map(&self.f)) }
    fn label(&self) -> String { format!("mapped({})", self.inner.label()) }
}

fn evens_then_odds(v: &[i32]) -> impl Iterator<Item = &i32> + '_ { v.iter().filter(|x| *x % 2 == 0).chain(v.iter().filter(|x| *x % 2 != 0)) }

fn choose<'a>(v: &'a [i32], mode: u8) -> Box<dyn Iterator<Item = i32> + 'a> {
    match mode {
        0 => Box::new(v.iter().copied()),
        1 => Box::new(v.iter().rev().copied()),
        2 => Box::new(v.iter().copied().filter(|x| x % 3 == 0)),
        3 => Box::new(v.windows(2).map(|w| w[1] - w[0])),
        _ => Box::new(std::iter::empty()),
    }
}

fn tree_walk(depth: u32) -> Box<dyn Iterator<Item = String>> {
    if depth == 0 { return Box::new(std::iter::once(String::from("leaf"))); }
    Box::new(tree_walk(depth - 1).map(|s| format!("L({})", s)).chain(std::iter::once(format!("n{}", depth))).chain(tree_walk(depth - 1).map(|s| format!("R({})", s)).take(1)))
}

fn group_by_len<'a, I: IntoIterator<Item = &'a str>>(it: I) -> BTreeMap<usize, Vec<&'a str>> {
    let mut m: BTreeMap<usize, Vec<&str>> = BTreeMap::new();
    for s in it { m.entry(s.len()).or_default().push(s); }
    m
}

fn main() {
    let r = RangeSrc { lo: -5, hi: 20, step: 4 };
    let v = VecSrc { v: vec![9, -2, 7, 7, 0] };
    let f = Fib { n: 15 };
    let sources: Vec<&dyn Source<i32>> = vec![&r, &v];
    for s in &sources {
        println!("{} -> {:?} pos={}", s.label(), s.items().collect::<Vec<_>>(), s.count_where(&|x| *x > 0));
    }
    println!("{} -> {:?}", f.label(), f.items().collect::<Vec<_>>());
    println!("sorted {:?}", v.collect_sorted());
    let m = Mapped { inner: Fib { n: 10 }, f: |x: u64| format!("<{}>", x * x), _p: std::marker::PhantomData };
    println!("{} -> {}", m.label(), m.items().collect::<Vec<_>>().concat());
    let mm = Mapped { inner: Mapped { inner: RangeSrc { lo: 0, hi: 5, step: 1 }, f: |x: i32| x * 10, _p: std::marker::PhantomData }, f: |x: i32| x as f64 / 4.0, _p: std::marker::PhantomData };
    println!("{} -> {:?}", mm.label(), mm.items().collect::<Vec<_>>());
    let boxed: Box<dyn Source<String>> = Box::new(VecSrc { v: vec!["b".to_string(), "a".to_string()] });
    println!("{} count={}", boxed.label(), boxed.count_where(&|s| s.as_str() > "a"));
    let data = [4, 9, 1, 6, 3, 8, 12, 5];
    println!("{:?}", evens_then_odds(&data).collect::<Vec<_>>());
    for mode in 0..5 { println!("mode {} {:?}", mode, choose(&data, mode).collect::<Vec<_>>()); }
    let total: i32 = (0..4).flat_map(|m| choose(&data, m)).sum();
    println!("total {}", total);
    println!("{:?}", tree_walk(2).collect::<Vec<_>>());
    let words = "the quick brown fox jumps over the lazy dog again".split(' ');
    println!("{:?}", group_by_len(words));
    let mut its: Vec<Box<dyn Iterator<Item = u32>>> = vec![Box::new(1..4), Box::new(vec![10, 20].into_iter()), Box::new(std::iter::repeat(7).take(2))];
    let mut round = Vec::new();
    loop {
        let mut any = false;
        for it in its.iter_mut() { if let Some(x) = it.next() { round.push(x); any = true; } }
        if !any { break; }
    }
    println!("interleaved {:?}", round);
    let mut peek: std::iter::Peekable<Box<dyn Iterator<Item = i32>>> = choose(&data, 1).peekable();
    while let Some(x) = peek.next() {
        if let Some(&nx) = peek.peek() { print!("{}{}", x, if nx > x { "<" } else { ">" }); } else { println!("{}", x); }
    }
    let sum_sq: i64 = (1..=10).map(|x: i64| x * x).filter(|x| x % 2 == 1).rev().skip(1).sum();
    println!("sum_sq {}", sum_sq);
    let nested: Vec<Vec<i32>> = vec![vec![1, 2], vec![], vec![3], vec![4, 5, 6]];
    let flat: Vec<i32> = nested.iter().flatten().copied().rev().collect();
    let flat2: Vec<(usize, i32)> = nested.into_iter().enumerate().flat_map(|(i, v)| v.into_iter().map(move |x| (i, x))).collect();
    println!("{:?} {:?}", flat, flat2);
    let opts = vec![Some(1), None, Some(3)];
    let s: i32 = opts.iter().flatten().sum();
    let cnt = opts.iter().filter_map(|x| x.as_ref()).count();
    println!("{} {}", s, cnt);
}
