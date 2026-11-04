use std::collections::HashMap;
use std::fmt::Debug;

trait Parser<'a> {
    type Out;
    fn parse(&self, s: &'a str) -> Option<(Self::Out, &'a str)>;
}
struct Word;
impl<'a> Parser<'a> for Word {
    type Out = &'a str;
    fn parse(&self, s: &'a str) -> Option<(&'a str, &'a str)> {
        let s = s.trim_start();
        let end = s.find(' ').unwrap_or(s.len());
        if end == 0 { None } else { Some((&s[..end], &s[end..])) }
    }
}
struct Num;
impl<'a> Parser<'a> for Num {
    type Out = i64;
    fn parse(&self, s: &'a str) -> Option<(i64, &'a str)> {
        let (w, rest) = Word.parse(s)?;
        Some((w.parse().ok()?, rest))
    }
}
struct Many<P>(P);
impl<'a, P: Parser<'a>> Parser<'a> for Many<P> {
    type Out = Vec<P::Out>;
    fn parse(&self, mut s: &'a str) -> Option<(Vec<P::Out>, &'a str)> {
        let mut v = vec![];
        while let Some((x, r)) = self.0.parse(s) { v.push(x); s = r; }
        Some((v, s))
    }
}
struct Map<P, F>(P, F);
impl<'a, P: Parser<'a>, F: Fn(P::Out) -> U, U> Parser<'a> for Map<P, F> {
    type Out = U;
    fn parse(&self, s: &'a str) -> Option<(U, &'a str)> {
        self.0.parse(s).map(|(o, r)| ((self.1)(o), r))
    }
}

fn words_of<P>(p: &P, text: &str) -> Vec<String>
where
    for<'a> P: Parser<'a, Out = &'a str>,
{
    let mut out = vec![];
    let mut s = text;
    while let Some((w, r)) = p.parse(s) { out.push(w.to_uppercase()); s = r; }
    out
}

fn parse_owned<P>(p: &P, text: String) -> Option<<P as Parser<'static>>::Out>
where
    for<'a> P: Parser<'a>,
    for<'a> <P as Parser<'a>>::Out: Debug,
{
    let leaked: &'static str = Box::leak(text.into_boxed_str());
    p.parse(leaked).map(|(o, _)| o)
}

struct Getter<F> where F: for<'a> Fn(&'a Rec) -> &'a str { f: F }
struct Rec { name: String, city: String }
impl<F> Getter<F> where F: for<'a> Fn(&'a Rec) -> &'a str {
    fn get<'r>(&self, r: &'r Rec) -> &'r str { (self.f)(r) }
}

trait Transform { fn apply(&self, x: i32) -> i32; }
impl<F: Fn(i32) -> i32> Transform for F { fn apply(&self, x: i32) -> i32 { self(x) } }
fn double(x: i32) -> i32 { x * 2 }

trait Handler<E> { fn handle(&mut self, e: &E) -> String; }
struct Logger { count: usize }
struct Click(i32, i32);
struct Key(char);
impl Handler<Click> for Logger { fn handle(&mut self, e: &Click) -> String { self.count += 1; format!("click {},{}", e.0, e.1) } }
impl Handler<Key> for Logger { fn handle(&mut self, e: &Key) -> String { self.count += 1; format!("key {}", e.0) } }
fn dispatch<E>(hs: &mut [Box<dyn Handler<E>>], e: E) -> Vec<String> { hs.iter_mut().map(|h| h.handle(&e)).collect() }

struct Pairs<I: Iterator> { it: I, prev: Option<I::Item> }
impl<I: Iterator> Iterator for Pairs<I> where I::Item: Clone {
    type Item = (I::Item, I::Item);
    fn next(&mut self) -> Option<Self::Item> {
        if self.prev.is_none() { self.prev = self.it.next(); }
        let p = self.prev.take()?;
        let n = self.it.next()?;
        self.prev = Some(n.clone());
        Some((p, n))
    }
}
trait IterExt: Iterator + Sized { fn pairs(self) -> Pairs<Self> { Pairs { it: self, prev: None } } }
impl<I: Iterator> IterExt for I {}

struct Countdown(u32);
impl Iterator for Countdown {
    type Item = u32;
    fn next(&mut self) -> Option<u32> { if self.0 == 0 { None } else { self.0 -= 1; Some(self.0 + 1) } }
    fn size_hint(&self) -> (usize, Option<usize>) { (self.0 as usize, Some(self.0 as usize)) }
}
impl ExactSizeIterator for Countdown {}
impl DoubleEndedIterator for Countdown {
    fn next_back(&mut self) -> Option<u32> { None }
}

fn memo_fib() -> impl FnMut(u64) -> u64 {
    let mut cache: HashMap<u64, u64> = HashMap::new();
    fn fib(n: u64, c: &mut HashMap<u64, u64>) -> u64 {
        if n < 2 { return n; }
        if let Some(v) = c.get(&n) { return *v; }
        let r = fib(n - 1, c) + fib(n - 2, c);
        c.insert(n, r);
        r
    }
    move |n| fib(n, &mut cache)
}

fn main() {
    println!("{:?}", Many(Num).parse("1 2 3 x"));
    println!("{:?}", Map(Many(Word), |v: Vec<&str>| v.len()).parse("a b c"));
    println!("{:?}", words_of(&Word, "hello big world"));
    println!("{:?}", parse_owned(&Many(Num), "4 5".to_string()));
    let g = Getter { f: |r: &Rec| r.city.as_str() };
    let g2 = Getter { f: |r: &Rec| &r.name[..1] };
    let rec = Rec { name: "Zed".into(), city: "Oslo".into() };
    println!("{} {}", g.get(&rec), g2.get(&rec));
    let ts: Vec<Box<dyn Transform>> = vec![Box::new(double), Box::new(|x| x + 1), Box::new(double as fn(i32) -> i32)];
    println!("{:?}", ts.iter().map(|t| t.apply(5)).collect::<Vec<_>>());
    println!("{}", (|x: i32| x - 1).apply(0));
    let mut hs: Vec<Box<dyn Handler<Click>>> = vec![Box::new(Logger { count: 0 })];
    println!("{:?}", dispatch(&mut hs, Click(1, 2)));
    let mut ks: Vec<Box<dyn Handler<Key>>> = vec![Box::new(Logger { count: 0 }), Box::new(Logger { count: 5 })];
    println!("{:?}", dispatch(&mut ks, Key('k')));
    let mut lg = Logger { count: 0 };
    <Logger as Handler<Key>>::handle(&mut lg, &Key('a'));
    Handler::<Click>::handle(&mut lg, &Click(0, 0));
    println!("{}", lg.count);
    println!("{:?}", (1..5).pairs().collect::<Vec<_>>());
    println!("{:?}", "abc".chars().pairs().map(|(a, b)| format!("{}{}", a, b)).collect::<Vec<_>>());
    let cd = Countdown(4);
    println!("{} {:?}", cd.len(), cd.rev().collect::<Vec<_>>());
    println!("{:?}", Countdown(3).pairs().collect::<Vec<_>>());
    let mut f = memo_fib();
    println!("{} {}", f(50), f(90));
}
