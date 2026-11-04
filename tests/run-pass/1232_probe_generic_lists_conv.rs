mod d01 {
    use std::fmt;
    enum List<T> { Cons(T, Box<List<T>>), Nil }
    use List::{Cons, Nil};
    impl<T: fmt::Display> fmt::Display for List<T> {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self { Cons(h, t) => { write!(f, "{}", h)?; if let Cons(..) = **t { write!(f, ", ")?; } write!(f, "{}", t) } Nil => Ok(()) }
        }
    }
    impl<T> List<T> {
        fn from_vec(v: Vec<T>) -> Self { v.into_iter().rev().fold(Nil, |acc, x| Cons(x, Box::new(acc))) }
        fn map<U, F: Fn(&T) -> U>(&self, f: &F) -> List<U> { match self { Cons(h, t) => Cons(f(h), Box::new(t.map(f))), Nil => Nil } }
        fn len(&self) -> usize { match self { Cons(_, t) => 1 + t.len(), Nil => 0 } }
    }
    struct MyMap<I, F> { it: I, f: F }
    impl<B, I: Iterator, F: FnMut(I::Item) -> B> Iterator for MyMap<I, F> { type Item = B; fn next(&mut self) -> Option<B> { self.it.next().map(&mut self.f) } }
    struct Pairs<I: Iterator> { it: I }
    impl<I: Iterator> Iterator for Pairs<I> { type Item = (I::Item, I::Item); fn next(&mut self) -> Option<Self::Item> { let a = self.it.next()?; let b = self.it.next()?; Some((a, b)) } }
    trait IterExt: Iterator + Sized {
        fn my_map<B, F: FnMut(Self::Item) -> B>(self, f: F) -> MyMap<Self, F> { MyMap { it: self, f } }
        fn pairs(self) -> Pairs<Self> { Pairs { it: self } }
        fn sum_by<K: std::ops::Add<Output = K> + Default, F: Fn(&Self::Item) -> K>(self, f: F) -> K { self.fold(K::default(), |a, x| a + f(&x)) }
    }
    impl<I: Iterator> IterExt for I {}
    trait Source: Iterator<Item = u8> { fn label(&self) -> String; }
    struct Bytes(std::vec::IntoIter<u8>);
    impl Iterator for Bytes { type Item = u8; fn next(&mut self) -> Option<u8> { self.0.next() } }
    impl Source for Bytes { fn label(&self) -> String { format!("bytes({})", self.0.len()) } }
    struct Memo<F: Fn(u64) -> u64> { f: F, cache: std::collections::HashMap<u64, u64>, calls: u32 }
    impl<F: Fn(u64) -> u64> Memo<F> { fn get(&mut self, k: u64) -> u64 { if let Some(&v) = self.cache.get(&k) { return v; } self.calls += 1; let v = (self.f)(k); self.cache.insert(k, v); v } }
    pub fn main() {
        let l = List::from_vec(vec![1, 2, 3]);
        println!("[{}] {}", l, l.len());
        let l2 = l.map(&|x| format!("<{}>", x * 10));
        println!("[{}]", l2);
        let e: List<u8> = Nil;
        println!("[{}]", e);
        let v: Vec<String> = (1..=5).my_map(|x| x * x).my_map(|x| x.to_string()).collect();
        println!("{:?}", v);
        println!("{:?}", "abcdefg".chars().pairs().collect::<Vec<_>>());
        println!("{}", vec!["aa", "b", "cccc"].into_iter().sum_by(|s| s.len()));
        println!("{}", (1..4).sum_by(|x| *x as f64 * 0.5));
        let mut src: Box<dyn Source> = Box::new(Bytes(vec![1, 2, 3, 4].into_iter()));
        let first = src.next();
        println!("{:?} {}", first, src.label());
        let rest: u32 = src.by_ref().map(|b| b as u32).sum();
        println!("{} {}", rest, src.label());
        let mut m = Memo { f: |k| (1..=k).product(), cache: Default::default(), calls: 0 };
        println!("{} {} {} {}", m.get(10), m.get(5), m.get(10), m.calls);
    }
}

mod d02 {
    use std::collections::BTreeMap;
    #[derive(Debug, Clone, PartialEq)]
    enum Val { I(i64), S(String), L(Vec<Val>), M(BTreeMap<String, Val>) }
    impl From<i64> for Val { fn from(i: i64) -> Self { Val::I(i) } }
    impl From<&str> for Val { fn from(s: &str) -> Self { Val::S(s.to_string()) } }
    impl<T: Into<Val>> From<Vec<T>> for Val { fn from(v: Vec<T>) -> Self { Val::L(v.into_iter().map(Into::into).collect()) } }
    impl TryFrom<Val> for i64 { type Error = String; fn try_from(v: Val) -> Result<i64, String> { match v { Val::I(i) => Ok(i), other => Err(format!("not int: {:?}", other)) } } }
    impl std::str::FromStr for Val {
        type Err = String;
        fn from_str(s: &str) -> Result<Val, String> {
            if let Ok(i) = s.parse::<i64>() { return Ok(Val::I(i)); }
            if let Some(inner) = s.strip_prefix('[').and_then(|r| r.strip_suffix(']')) {
                if inner.is_empty() { return Ok(Val::L(vec![])); }
                return inner.split(',').map(|p| p.trim().parse()).collect::<Result<Vec<Val>, _>>().map(Val::L);
            }
            if s.chars().all(|c| c.is_alphanumeric()) { Ok(Val::S(s.into())) } else { Err(format!("bad: {}", s)) }
        }
    }
    pub fn main() {
        let v: Val = vec![1i64, 2, 3].into();
        let s: Val = "hi".into();
        println!("{:?} {:?}", v, s);
        let i: Result<i64, _> = Val::I(5).try_into();
        let j: Result<i64, _> = i64::try_from(Val::S("x".into()));
        println!("{:?} {:?}", i, j);
        for t in ["42", "[1, 2, [3]]", "[1, x]", "abc", "a-b", "[]"] { println!("{:?}", t.parse::<Val>()); }
        let mut m = BTreeMap::new();
        m.insert("k".to_string(), Val::from(vec!["a", "b"]));
        let mv = Val::M(m);
        println!("{:?}", mv);
        let n: i64 = 7u8.into();
        let f: f64 = 3i32.into();
        let c: u32 = 'a'.into();
        let ch = char::try_from(98u32);
        let st: String = 'z'.into();
        let bx: Box<str> = "b".into();
        let o: Option<u8> = 5.into();
        println!("{} {} {} {:?} {} {} {:?}", n, f, c, ch, st, bx, o);
    }
}

fn main() {
    println!("== d01");
    d01::main();
    println!("== d02");
    d02::main();
}
