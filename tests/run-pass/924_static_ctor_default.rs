use std::collections::{BTreeMap, VecDeque};
use std::fmt::Debug;

trait Make { fn make(seed: u32) -> Self; }
impl Make for u32 { fn make(s: u32) -> Self { s * 2 } }
impl Make for String { fn make(s: u32) -> Self { "s".repeat(s as usize) } }
impl<T: Make> Make for Vec<T> { fn make(s: u32) -> Self { (0..s).map(T::make).collect() } }
impl<A: Make, B: Make> Make for (A, B) { fn make(s: u32) -> Self { (A::make(s), B::make(s + 1)) } }
impl<T: Make> Make for Option<T> { fn make(s: u32) -> Self { if s == 0 { None } else { Some(T::make(s)) } } }

fn build<T: Make + Debug>(s: u32) -> String { format!("{:?}", T::make(s)) }

#[derive(Debug, Default, Clone, PartialEq)]
struct Stats<T> { count: usize, total: T, history: Vec<T> }

impl<T: Default + Clone + std::ops::AddAssign> Stats<T> {
    fn record(&mut self, x: T) { self.count += 1; self.total += x.clone(); self.history.push(x); }
}

#[derive(Debug, Default)]
struct Registry<K: Ord, V> { items: BTreeMap<K, V>, queue: VecDeque<K>, name: Option<String> }

fn fresh<T: Default>() -> T { T::default() }
fn reset<T: Default>(t: &mut T) -> T { std::mem::take(t) }
fn defaults_of<A: Default + Debug, B: Default + Debug, C: Default + Debug>() -> String {
    format!("{:?} {:?} {:?}", A::default(), B::default(), C::default())
}

fn main() {
    println!("{}", build::<u32>(5));
    println!("{}", build::<String>(3));
    println!("{}", build::<Vec<u32>>(4));
    println!("{}", build::<(String, Vec<String>)>(2));
    println!("{}", build::<Option<(u32, Option<u32>)>>(1));
    println!("{}", build::<Option<u32>>(0));
    let mut s: Stats<i64> = Stats::default();
    for x in [5, -2, 10] { s.record(x); }
    println!("{:?}", s);
    let mut fs: Stats<f32> = fresh();
    fs.record(0.5); fs.record(0.25);
    println!("{:?}", fs);
    let old = reset(&mut fs);
    println!("{:?} {:?} {}", old.total, fs, fs == Stats::default());
    let mut r: Registry<String, Vec<u8>> = Default::default();
    r.items.insert("k".into(), vec![1]);
    r.queue.push_back("k".into());
    println!("{:?}", r);
    println!("{}", defaults_of::<i8, String, (bool, char, Option<u8>)>());
    println!("{}", defaults_of::<Vec<i32>, [u8; 3], Stats<u16>>());
    let v: Vec<Stats<u8>> = (0..2).map(|_| fresh()).collect();
    println!("{:?}", v);
}
