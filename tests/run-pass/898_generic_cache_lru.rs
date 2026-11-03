use std::collections::HashMap;
use std::hash::Hash;
use std::fmt::Debug;

struct Lru<K, V> {
    cap: usize,
    map: HashMap<K, (V, u64)>,
    tick: u64,
    hits: u32,
    misses: u32,
}

impl<K: Hash + Eq + Clone + Ord + Debug, V: Clone + Debug> Lru<K, V> {
    fn new(cap: usize) -> Self { Lru { cap, map: HashMap::new(), tick: 0, hits: 0, misses: 0 } }
    fn get(&mut self, k: &K) -> Option<V> {
        self.tick += 1;
        let t = self.tick;
        match self.map.get_mut(k) {
            Some(e) => { e.1 = t; self.hits += 1; Some(e.0.clone()) }
            None => { self.misses += 1; None }
        }
    }
    fn put(&mut self, k: K, v: V) -> Option<K> {
        self.tick += 1;
        let mut evicted = None;
        if !self.map.contains_key(&k) && self.map.len() >= self.cap {
            let oldest = self.map.iter().min_by_key(|(_, (_, t))| *t).map(|(k, _)| k.clone()).unwrap();
            self.map.remove(&oldest);
            evicted = Some(oldest);
        }
        self.map.insert(k, (v, self.tick));
        evicted
    }
    fn get_or_compute<F: FnOnce(&K) -> V>(&mut self, k: K, f: F) -> V {
        if let Some(v) = self.get(&k) { return v; }
        let v = f(&k);
        self.put(k, v.clone());
        v
    }
    fn keys_sorted(&self) -> Vec<K> { let mut v: Vec<K> = self.map.keys().cloned().collect(); v.sort(); v }
}

struct Memo<A, R> {
    table: HashMap<A, R>,
    f: fn(&mut Memo<A, R>, A) -> R,
    calls: usize,
}
impl<A: Hash + Eq + Clone, R: Clone> Memo<A, R> {
    fn new(f: fn(&mut Memo<A, R>, A) -> R) -> Self { Memo { table: HashMap::new(), f, calls: 0 } }
    fn call(&mut self, a: A) -> R {
        if let Some(r) = self.table.get(&a) { return r.clone(); }
        self.calls += 1;
        let f = self.f;
        let r = f(self, a.clone());
        self.table.insert(a, r.clone());
        r
    }
}

fn main() {
    let mut c: Lru<&str, i32> = Lru::new(2);
    println!("{:?}", c.put("a", 1));
    println!("{:?}", c.put("b", 2));
    println!("{:?}", c.get(&"a"));
    println!("{:?}", c.put("c", 3));
    println!("{:?} {:?}", c.get(&"b"), c.get(&"c"));
    println!("{:?}", c.put("d", 4));
    println!("{:?} hits={} misses={}", c.keys_sorted(), c.hits, c.misses);
    let mut sq: Lru<u64, String> = Lru::new(3);
    for k in [1, 2, 1, 3, 4, 1, 2] {
        let v = sq.get_or_compute(k, |k| format!("sq{}", k * k));
        print!("{} ", v);
    }
    println!();
    println!("{:?} {} {}", sq.keys_sorted(), sq.hits, sq.misses);
    let mut fib = Memo::new(|m: &mut Memo<u64, u64>, n: u64| if n < 2 { n } else { m.call(n - 1) + m.call(n - 2) });
    println!("{} calls={}", fib.call(80), fib.calls);
    let mut paths = Memo::new(|m: &mut Memo<(u32, u32), u128>, (r, c): (u32, u32)| {
        if r == 0 || c == 0 { 1 } else { m.call((r - 1, c)) + m.call((r, c - 1)) }
    });
    println!("{} calls={}", paths.call((16, 16)), paths.calls);
}
