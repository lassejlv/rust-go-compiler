use std::ops::{Deref, DerefMut};
use std::fmt::Debug;

struct Logged<T: Debug> {
    inner: T,
    reads: std::cell::Cell<u32>,
    writes: u32,
}

impl<T: Debug> Logged<T> {
    fn new(t: T) -> Self { Logged { inner: t, reads: std::cell::Cell::new(0), writes: 0 } }
    fn stats(&self) -> (u32, u32) { (self.reads.get(), self.writes) }
}

impl<T: Debug> Deref for Logged<T> {
    type Target = T;
    fn deref(&self) -> &T { self.reads.set(self.reads.get() + 1); &self.inner }
}
impl<T: Debug> DerefMut for Logged<T> {
    fn deref_mut(&mut self) -> &mut T { self.writes += 1; &mut self.inner }
}

struct NonEmpty<T> { head: T, tail: Vec<T> }
impl<T> NonEmpty<T> {
    fn new(h: T) -> Self { NonEmpty { head: h, tail: vec![] } }
    fn first(&self) -> &T { &self.head }
}

struct Sorted<T: Ord>(Vec<T>);
impl<T: Ord> Sorted<T> {
    fn from_vec(mut v: Vec<T>) -> Self { v.sort(); Sorted(v) }
    fn insert(&mut self, x: T) {
        let pos = self.0.binary_search(&x).unwrap_or_else(|e| e);
        self.0.insert(pos, x);
    }
}
impl<T: Ord> Deref for Sorted<T> {
    type Target = [T];
    fn deref(&self) -> &[T] { &self.0 }
}

fn total_len<S: Deref<Target = str>>(xs: &[S]) -> usize { xs.iter().map(|s| s.len()).sum() }

fn print_slice<T: Debug>(s: &[T]) { println!("slice {:?}", s); }

fn main() {
    let mut l = Logged::new(vec![1, 2, 3]);
    println!("{}", l.len());
    println!("{:?}", l.iter().map(|x| x * 2).collect::<Vec<_>>());
    l.push(4);
    l[0] = 100;
    println!("{:?}", *l);
    println!("{:?}", l.stats());
    let mut ls = Logged::new(String::from("hi"));
    ls.push_str(" there");
    println!("{} {} {:?}", ls.to_uppercase(), ls.len(), ls.stats());

    let mut ne = NonEmpty::new("a");
    ne.tail.push("b");
    println!("{} {}", ne.first(), ne.tail.len());

    let mut s = Sorted::from_vec(vec![5, 1, 4]);
    s.insert(3);
    s.insert(0);
    s.insert(9);
    println!("{:?} first={:?} contains4={} len={}", &*s, s.first(), s.contains(&4), s.len());
    print_slice(&s);
    let max = s.iter().max();
    println!("{:?} {:?}", max, s.binary_search(&4));

    let owned = vec![String::from("abc"), String::from("de")];
    println!("{}", total_len(&owned));
    let boxed: Vec<Box<str>> = vec!["xyz".into(), "w".into()];
    println!("{}", total_len(&boxed));
}
