use std::collections::hash_map::DefaultHasher;
use std::collections::BTreeMap;
use std::hash::{BuildHasher, BuildHasherDefault, Hash, Hasher};
use std::marker::PhantomData;
use std::ops::Add;

// A map type with a default hasher parameter, like hashbrown / std HashMap<K, V, S = RandomState>.
#[derive(Default, Clone, Copy)]
struct SimpleState;

struct SimpleHasher(u64);
impl Hasher for SimpleHasher {
    fn finish(&self) -> u64 {
        self.0
    }
    fn write(&mut self, bytes: &[u8]) {
        for b in bytes {
            self.0 = self.0.wrapping_mul(31).wrapping_add(*b as u64);
        }
    }
}
impl BuildHasher for SimpleState {
    type Hasher = SimpleHasher;
    fn build_hasher(&self) -> SimpleHasher {
        SimpleHasher(7)
    }
}

struct BucketMap<K, V, S = SimpleState> {
    buckets: Vec<Vec<(K, V)>>,
    hasher: S,
    len: usize,
}

impl<K: Hash + Eq, V> BucketMap<K, V> {
    fn new(n: usize) -> Self {
        Self::with_hasher(n, SimpleState)
    }
}

impl<K: Hash + Eq, V, S: BuildHasher> BucketMap<K, V, S> {
    fn with_hasher(n: usize, hasher: S) -> Self {
        BucketMap { buckets: (0..n).map(|_| Vec::new()).collect(), hasher, len: 0 }
    }
    fn bucket(&self, k: &K) -> usize {
        let mut h = self.hasher.build_hasher();
        k.hash(&mut h);
        (h.finish() % self.buckets.len() as u64) as usize
    }
    fn insert(&mut self, k: K, v: V) -> Option<V> {
        let b = self.bucket(&k);
        for e in self.buckets[b].iter_mut() {
            if e.0 == k {
                return Some(std::mem::replace(&mut e.1, v));
            }
        }
        self.buckets[b].push((k, v));
        self.len += 1;
        None
    }
    fn get(&self, k: &K) -> Option<&V> {
        let b = self.bucket(k);
        self.buckets[b].iter().find(|e| &e.0 == k).map(|e| &e.1)
    }
    fn bucket_sizes(&self) -> Vec<usize> {
        self.buckets.iter().map(|b| b.len()).collect()
    }
}

// Default type params on traits: Rhs = Self
#[derive(Debug, Clone, Copy, PartialEq)]
struct Meters(f64);
#[derive(Debug, Clone, Copy, PartialEq)]
struct Millis(f64);

impl Add for Meters {
    type Output = Meters;
    fn add(self, o: Meters) -> Meters {
        Meters(self.0 + o.0)
    }
}
impl Add<Millis> for Meters {
    type Output = Meters;
    fn add(self, o: Millis) -> Meters {
        Meters(self.0 + o.0 / 1000.0)
    }
}

trait Combine<Rhs = Self> {
    type Out;
    fn combine(&self, r: &Rhs) -> Self::Out;
}
impl Combine for String {
    type Out = String;
    fn combine(&self, r: &String) -> String {
        format!("{}{}", self, r)
    }
}
impl Combine<usize> for String {
    type Out = String;
    fn combine(&self, r: &usize) -> String {
        self.repeat(*r)
    }
}

// Default params with PhantomData unit marker
struct Id<T, Tag = ()> {
    raw: u32,
    _p: PhantomData<(T, Tag)>,
}
impl<T, Tag> Id<T, Tag> {
    fn new(raw: u32) -> Self {
        Id { raw, _p: PhantomData }
    }
}
struct Admin;
fn describe_id<T>(id: &Id<T>) -> String {
    format!("plain id {}", id.raw)
}
fn describe_admin<T>(id: &Id<T, Admin>) -> String {
    format!("admin id {}", id.raw)
}

type FastMap<K, V> = BucketMap<K, V, BuildHasherDefault<DefaultHasher>>;

fn main() {
    let mut m: BucketMap<&str, i32> = BucketMap::new(4);
    for (i, w) in ["one", "two", "three", "four", "five", "six"].iter().enumerate() {
        m.insert(w, i as i32 + 1);
    }
    println!("{:?}", m.insert("two", 22));
    println!("{:?} {:?} {:?}", m.get(&"two"), m.get(&"six"), m.get(&"seven"));
    println!("len={} buckets={:?}", m.len, m.bucket_sizes());

    let mut f: FastMap<u32, String> = BucketMap::with_hasher(3, Default::default());
    for i in 0..5 {
        f.insert(i * 10, format!("v{}", i));
    }
    let mut found = BTreeMap::new();
    for i in 0..6 {
        found.insert(i * 10, f.get(&(i * 10)).cloned());
    }
    println!("{:?} total={}", found, f.bucket_sizes().iter().sum::<usize>());

    println!("{:?}", Meters(1.5) + Meters(2.0));
    println!("{:?}", Meters(1.0) + Millis(250.0));
    let s = String::from("ab");
    println!("{} {}", s.combine(&String::from("cd")), s.combine(&3usize));

    let a: Id<String> = Id::new(5);
    let b: Id<String, Admin> = Id::new(6);
    println!("{} / {}", describe_id(&a), describe_admin(&b));
}
