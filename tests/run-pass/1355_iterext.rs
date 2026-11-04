// A mini "itertools": extension traits with blanket impls, hand-written
// adapter structs (generic over iterators and closures), DoubleEndedIterator
// and ExactSizeIterator impls, a lending-style windows API via GATs, a
// typestate builder with PhantomData, and a string interner with lifetimes.
use std::collections::{BTreeMap, HashMap, VecDeque};
use std::fmt::{self, Debug, Display};
use std::hash::Hash;
use std::marker::PhantomData;

// ---------- adapters ----------
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
}

struct Batching<I, F> {
    it: I,
    f: F,
}

impl<I: Iterator, B, F: FnMut(&mut I) -> Option<B>> Iterator for Batching<I, F> {
    type Item = B;
    fn next(&mut self) -> Option<B> {
        (self.f)(&mut self.it)
    }
}

struct Dedup<I: Iterator> {
    it: std::iter::Peekable<I>,
}

impl<I: Iterator> Iterator for Dedup<I>
where
    I::Item: PartialEq,
{
    type Item = (usize, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        let first = self.it.next()?;
        let mut n = 1;
        while self.it.next_if(|x| *x == first).is_some() {
            n += 1;
        }
        Some((n, first))
    }
}

struct TupleWindows<I: Iterator> {
    it: I,
    last: Option<I::Item>,
}

impl<I> Iterator for TupleWindows<I>
where
    I: Iterator,
    I::Item: Clone,
{
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        let prev = match self.last.take() {
            Some(p) => p,
            None => self.it.next()?,
        };
        let cur = self.it.next()?;
        self.last = Some(cur.clone());
        Some((prev, cur))
    }
}

trait IterExt: Iterator + Sized {
    fn interleave<J: IntoIterator<Item = Self::Item>>(self, other: J) -> Interleave<Self, J::IntoIter> {
        Interleave { a: self, b: other.into_iter(), flag: false }
    }
    fn batching<B, F: FnMut(&mut Self) -> Option<B>>(self, f: F) -> Batching<Self, F> {
        Batching { it: self, f }
    }
    fn dedup_with_count(self) -> Dedup<Self>
    where
        Self::Item: PartialEq,
    {
        Dedup { it: self.peekable() }
    }
    fn tuple_windows(self) -> TupleWindows<Self>
    where
        Self::Item: Clone,
    {
        TupleWindows { it: self, last: None }
    }
    fn counts(self) -> HashMap<Self::Item, usize>
    where
        Self::Item: Hash + Eq,
    {
        let mut m = HashMap::new();
        for x in self {
            *m.entry(x).or_insert(0) += 1;
        }
        m
    }
    fn into_group_map_by<K: Ord, F: Fn(&Self::Item) -> K>(self, f: F) -> BTreeMap<K, Vec<Self::Item>> {
        let mut m: BTreeMap<K, Vec<Self::Item>> = BTreeMap::new();
        for x in self {
            m.entry(f(&x)).or_default().push(x);
        }
        m
    }
    fn join_with(mut self, sep: &str) -> String
    where
        Self::Item: Display,
    {
        let mut s = match self.next() {
            Some(x) => x.to_string(),
            None => return String::new(),
        };
        for x in self {
            s.push_str(sep);
            s += &x.to_string();
        }
        s
    }
    fn minmax(mut self) -> Option<(Self::Item, Self::Item)>
    where
        Self::Item: PartialOrd + Clone,
    {
        let first = self.next()?;
        Some(self.fold((first.clone(), first), |(lo, hi), x| {
            let lo = if x < lo { x.clone() } else { lo };
            let hi = if x > hi { x } else { hi };
            (lo, hi)
        }))
    }
    fn sorted_by_key<K: Ord>(self, f: impl FnMut(&Self::Item) -> K) -> std::vec::IntoIter<Self::Item> {
        let mut v: Vec<_> = self.collect();
        v.sort_by_key(f);
        v.into_iter()
    }
}

impl<I: Iterator> IterExt for I {}

// ---------- a custom double-ended exact-size iterator ----------
#[derive(Clone, Debug)]
struct Countdown {
    lo: u32,
    hi: u32, // exclusive
    step: u32,
}

impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> {
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
}

impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> {
        let n = self.len() as u32;
        if n == 0 {
            return None;
        }
        let last = self.lo + (n - 1) * self.step;
        self.hi = last;
        Some(last)
    }
}

impl ExactSizeIterator for Countdown {}

// ---------- GAT lending iterator ----------
trait Lending {
    type Item<'a>
    where
        Self: 'a;
    fn next<'a>(&'a mut self) -> Option<Self::Item<'a>>;
}

struct WindowsMut<'s, T> {
    data: &'s mut [T],
    pos: usize,
    size: usize,
}

impl<'s, T> Lending for WindowsMut<'s, T> {
    type Item<'a> = &'a mut [T] where Self: 'a;
    fn next<'a>(&'a mut self) -> Option<&'a mut [T]> {
        if self.pos + self.size > self.data.len() {
            return None;
        }
        let w = &mut self.data[self.pos..self.pos + self.size];
        self.pos += 1;
        Some(w)
    }
}

// ---------- typestate builder ----------
struct Missing;
struct Present;

struct RequestBuilder<U, M> {
    url: Option<String>,
    method: Option<&'static str>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    _s: PhantomData<(U, M)>,
}

#[derive(Debug)]
struct Request {
    url: String,
    method: &'static str,
    headers: Vec<(String, String)>,
    body: Option<String>,
}

impl RequestBuilder<Missing, Missing> {
    fn new() -> Self {
        RequestBuilder { url: None, method: None, headers: vec![], body: None, _s: PhantomData }
    }
}

impl<U, M> RequestBuilder<U, M> {
    fn header(mut self, k: &str, v: impl ToString) -> Self {
        self.headers.push((k.to_string(), v.to_string()));
        self
    }
    fn url(self, u: &str) -> RequestBuilder<Present, M> {
        RequestBuilder { url: Some(u.to_string()), method: self.method, headers: self.headers, body: self.body, _s: PhantomData }
    }
    fn method(self, m: &'static str) -> RequestBuilder<U, Present> {
        RequestBuilder { url: self.url, method: Some(m), headers: self.headers, body: self.body, _s: PhantomData }
    }
}

impl RequestBuilder<Present, Present> {
    fn body(mut self, b: impl Into<String>) -> Self {
        self.body = Some(b.into());
        self
    }
    fn build(self) -> Request {
        let mut headers = self.headers;
        if let Some(b) = &self.body {
            headers.push(("content-length".into(), b.len().to_string()));
        }
        headers.sort();
        Request { url: self.url.unwrap(), method: self.method.unwrap(), headers, body: self.body }
    }
}

// ---------- string interner ----------
#[derive(Default)]
struct Interner<'a> {
    map: HashMap<&'a str, u32>,
    names: Vec<&'a str>,
}

impl<'a> Interner<'a> {
    fn intern(&mut self, s: &'a str) -> u32 {
        if let Some(&id) = self.map.get(s) {
            return id;
        }
        let id = self.names.len() as u32;
        self.map.insert(s, id);
        self.names.push(s);
        id
    }
    fn resolve(&self, id: u32) -> Option<&'a str> {
        self.names.get(id as usize).copied()
    }
}

fn longest<'a, 'b: 'a>(a: &'a str, b: &'b str) -> &'a str {
    if b.len() > a.len() {
        b
    } else {
        a
    }
}

struct Wrapper<T>(Vec<T>);
impl<T: Debug> Display for Wrapper<T> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "[")?;
        for (i, x) in self.0.iter().enumerate() {
            if i > 0 {
                write!(f, "; ")?;
            }
            write!(f, "{:?}", x)?;
        }
        write!(f, "]")
    }
}

fn main() {
    let a = [1, 3, 5, 7];
    let b = vec![2, 4];
    println!("{}", a.iter().copied().interleave(b.clone()).join_with(","));
    let pairs: Vec<(i32, i32)> = (1..=7).batching(|it| Some((it.next()?, it.next().unwrap_or(0)))).collect();
    println!("{:?}", pairs);
    let s = "aaabccddddeaa";
    let runs: Vec<String> = s.chars().dedup_with_count().map(|(n, c)| if n > 1 { format!("{}{}", n, c) } else { c.to_string() }).collect();
    println!("rle: {}", runs.concat());
    let diffs: Vec<i32> = [1, 4, 9, 16, 25].iter().tuple_windows().map(|(a, b)| b - a).collect();
    println!("diffs {:?}", diffs);
    let mut counts: Vec<(char, usize)> = "mississippi".chars().counts().into_iter().collect();
    counts.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    println!("counts {:?}", counts);
    let words = ["apple", "bob", "cat", "avocado", "banana", "cherry", "axe"];
    let groups = words.iter().into_group_map_by(|w| w.chars().next().unwrap());
    println!("groups {:?}", groups);
    println!("minmax {:?} {:?} {:?}", [3.5, -1.0, 9.25, 0.0].iter().copied().minmax(), words.iter().minmax(), std::iter::empty::<u8>().minmax());
    println!("sorted {:?}", words.iter().sorted_by_key(|w| (w.len(), std::cmp::Reverse(**w))).collect::<Vec<_>>());

    let cd = Countdown { lo: 3, hi: 20, step: 4 };
    println!("len {} fwd {:?} rev {:?}", cd.len(), cd.clone().collect::<Vec<_>>(), cd.clone().rev().collect::<Vec<_>>());
    let mut mixed = cd.clone();
    println!("mixed {:?} {:?} {:?} {:?} len={}", mixed.next(), mixed.next_back(), mixed.next(), mixed.next_back(), mixed.len());
    println!("rposition {:?} nth_back {:?} zip-rev {:?}", cd.clone().rposition(|x| x % 2 == 1), cd.clone().nth_back(1), cd.clone().zip(cd.clone().rev()).collect::<Vec<_>>());
    println!("chain-rev {:?}", cd.clone().chain(100..102).rev().skip(1).step_by(2).collect::<Vec<u32>>());

    let mut data = [1, 2, 3, 4, 5, 6];
    let mut w = WindowsMut { data: &mut data, pos: 0, size: 3 };
    while let Some(win) = w.next() {
        win[2] += win[0] + win[1];
    }
    println!("lending windows -> {:?}", data);

    let req = RequestBuilder::new().header("accept", "*/*").method("POST").header("x-retry", 3).url("https://example.com/api").body("{\"k\":1}").build();
    println!("{:?}", req);
    let get = RequestBuilder::new().url("/health").method("GET").build();
    println!("{} {} {:?} {}", get.method, get.url, get.body, get.headers.len());

    let text = String::from("the quick brown fox jumps over the lazy dog the end");
    let mut int = Interner::default();
    let ids: Vec<u32> = text.split(' ').map(|w| int.intern(w)).collect();
    println!("ids {:?} unique {}", ids, int.names.len());
    println!("resolve {:?} {:?}", ids.iter().rev().take(3).map(|&i| int.resolve(i).unwrap()).collect::<Vec<_>>(), int.resolve(99));
    let l;
    {
        let s2 = "a much longer string here";
        l = longest(int.resolve(2).unwrap(), s2);
    }
    println!("longest {}", l);
    println!("{}", Wrapper(vec![Some(1), None]));
    println!("{}", Wrapper(vec!["a\tb", "c"]));
    let mut dq: VecDeque<i32> = (1..=6).collect();
    dq.rotate_left(2);
    dq.push_front(0);
    let (front, back) = dq.as_slices();
    println!("{:?} {} {:?}", dq, front.len() + back.len(), dq.iter().rev().step_by(2).collect::<Vec<_>>());
    let contiguous = dq.make_contiguous();
    contiguous.sort_unstable_by(|a, b| b.cmp(a));
    println!("{:?} {:?}", dq, dq.range(1..3).collect::<Vec<_>>());
}
