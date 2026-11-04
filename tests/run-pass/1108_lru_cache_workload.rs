// LRU cache implemented with an index-based doubly linked list + HashMap,
// a Rc<RefCell<>>-based variant, an LFU variant, and a deterministic
// benchmark-style workload comparing hit rates.
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};
use std::hash::Hash;
use std::rc::{Rc, Weak};

const NIL: usize = usize::MAX;

struct Entry<K, V> {
    key: K,
    val: V,
    prev: usize,
    next: usize,
}

pub struct Lru<K, V> {
    map: HashMap<K, usize>,
    slab: Vec<Entry<K, V>>,
    free: Vec<usize>,
    head: usize,
    tail: usize,
    cap: usize,
    hits: u64,
    misses: u64,
    evictions: Vec<K>,
}

impl<K: Hash + Eq + Clone, V> Lru<K, V> {
    pub fn new(cap: usize) -> Self {
        assert!(cap > 0);
        Lru { map: HashMap::with_capacity(cap), slab: Vec::with_capacity(cap), free: vec![], head: NIL, tail: NIL, cap, hits: 0, misses: 0, evictions: vec![] }
    }

    fn unlink(&mut self, i: usize) {
        let (p, n) = (self.slab[i].prev, self.slab[i].next);
        if p != NIL { self.slab[p].next = n; } else { self.head = n; }
        if n != NIL { self.slab[n].prev = p; } else { self.tail = p; }
    }

    fn push_front(&mut self, i: usize) {
        self.slab[i].prev = NIL;
        self.slab[i].next = self.head;
        if self.head != NIL { self.slab[self.head].prev = i; }
        self.head = i;
        if self.tail == NIL { self.tail = i; }
    }

    pub fn get(&mut self, k: &K) -> Option<&V> {
        match self.map.get(k).copied() {
            Some(i) => {
                self.hits += 1;
                self.unlink(i);
                self.push_front(i);
                Some(&self.slab[i].val)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    pub fn get_mut(&mut self, k: &K) -> Option<&mut V> {
        let i = *self.map.get(k)?;
        self.unlink(i);
        self.push_front(i);
        Some(&mut self.slab[i].val)
    }

    pub fn put(&mut self, k: K, v: V) -> Option<V> {
        if let Some(&i) = self.map.get(&k) {
            let old = std::mem::replace(&mut self.slab[i].val, v);
            self.unlink(i);
            self.push_front(i);
            return Some(old);
        }
        if self.map.len() == self.cap {
            let t = self.tail;
            self.unlink(t);
            let old_key = self.slab[t].key.clone();
            self.map.remove(&old_key);
            self.evictions.push(old_key);
            self.free.push(t);
        }
        let e = Entry { key: k.clone(), val: v, prev: NIL, next: NIL };
        let i = match self.free.pop() {
            Some(i) => {
                self.slab[i] = e;
                i
            }
            None => {
                self.slab.push(e);
                self.slab.len() - 1
            }
        };
        self.map.insert(k, i);
        self.push_front(i);
        None
    }

    pub fn remove(&mut self, k: &K) -> bool {
        match self.map.remove(k) {
            Some(i) => {
                self.unlink(i);
                self.free.push(i);
                true
            }
            None => false,
        }
    }

    pub fn keys_mru(&self) -> Vec<&K> {
        let mut out = Vec::new();
        let mut i = self.head;
        while i != NIL {
            out.push(&self.slab[i].key);
            i = self.slab[i].next;
        }
        out
    }

    pub fn len(&self) -> usize { self.map.len() }

    pub fn hit_rate(&self) -> f64 {
        let t = self.hits + self.misses;
        if t == 0 { 0.0 } else { self.hits as f64 / t as f64 }
    }
}

// ---- Rc/Weak linked-list LRU ----
struct Node {
    key: u32,
    val: String,
    prev: Option<Weak<RefCell<Node>>>,
    next: Option<Rc<RefCell<Node>>>,
}

struct RcLru {
    map: HashMap<u32, Rc<RefCell<Node>>>,
    head: Option<Rc<RefCell<Node>>>,
    tail: Option<Weak<RefCell<Node>>>,
    cap: usize,
}

impl RcLru {
    fn new(cap: usize) -> Self { RcLru { map: HashMap::new(), head: None, tail: None, cap } }

    fn detach(&mut self, n: &Rc<RefCell<Node>>) {
        let prev = n.borrow_mut().prev.take().and_then(|w| w.upgrade());
        let next = n.borrow_mut().next.take();
        match (&prev, &next) {
            (Some(p), Some(nx)) => {
                nx.borrow_mut().prev = Some(Rc::downgrade(p));
                p.borrow_mut().next = Some(nx.clone());
            }
            (Some(p), None) => {
                p.borrow_mut().next = None;
                self.tail = Some(Rc::downgrade(p));
            }
            (None, Some(nx)) => {
                nx.borrow_mut().prev = None;
                self.head = Some(nx.clone());
            }
            (None, None) => {
                self.head = None;
                self.tail = None;
            }
        }
    }

    fn attach_front(&mut self, n: Rc<RefCell<Node>>) {
        match self.head.take() {
            Some(h) => {
                h.borrow_mut().prev = Some(Rc::downgrade(&n));
                n.borrow_mut().next = Some(h);
            }
            None => self.tail = Some(Rc::downgrade(&n)),
        }
        self.head = Some(n);
    }

    fn get(&mut self, k: u32) -> Option<String> {
        let n = self.map.get(&k)?.clone();
        self.detach(&n);
        self.attach_front(n.clone());
        let v = n.borrow().val.clone();
        Some(v)
    }

    fn put(&mut self, k: u32, v: String) {
        if let Some(n) = self.map.get(&k).cloned() {
            n.borrow_mut().val = v;
            self.detach(&n);
            self.attach_front(n);
            return;
        }
        if self.map.len() == self.cap {
            if let Some(t) = self.tail.as_ref().and_then(|w| w.upgrade()) {
                self.detach(&t);
                let key = t.borrow().key;
                self.map.remove(&key);
            }
        }
        let n = Rc::new(RefCell::new(Node { key: k, val: v, prev: None, next: None }));
        self.map.insert(k, n.clone());
        self.attach_front(n);
    }

    fn order(&self) -> Vec<u32> {
        let mut out = vec![];
        let mut cur = self.head.clone();
        while let Some(n) = cur {
            out.push(n.borrow().key);
            cur = n.borrow().next.clone();
        }
        out
    }

    fn order_rev(&self) -> Vec<u32> {
        let mut out = vec![];
        let mut cur = self.tail.as_ref().and_then(|w| w.upgrade());
        while let Some(n) = cur {
            out.push(n.borrow().key);
            cur = n.borrow().prev.as_ref().and_then(|w| w.upgrade());
        }
        out
    }
}

// ---- LFU (min frequency, ties broken by age) ----
struct Lfu {
    cap: usize,
    vals: HashMap<u64, (u64, u32, u64)>, // key -> (value, freq, last_tick)
    by_freq: BTreeMap<(u32, u64), u64>,  // (freq, tick) -> key
    tick: u64,
    hits: u64,
    misses: u64,
}

impl Lfu {
    fn new(cap: usize) -> Self { Lfu { cap, vals: HashMap::new(), by_freq: BTreeMap::new(), tick: 0, hits: 0, misses: 0 } }
    fn touch(&mut self, k: u64) {
        self.tick += 1;
        let e = self.vals.get_mut(&k).unwrap();
        self.by_freq.remove(&(e.1, e.2));
        e.1 += 1;
        e.2 = self.tick;
        self.by_freq.insert((e.1, e.2), k);
    }
    fn get(&mut self, k: u64) -> Option<u64> {
        if self.vals.contains_key(&k) {
            self.hits += 1;
            self.touch(k);
            Some(self.vals[&k].0)
        } else {
            self.misses += 1;
            None
        }
    }
    fn put(&mut self, k: u64, v: u64) {
        if self.vals.contains_key(&k) {
            self.vals.get_mut(&k).unwrap().0 = v;
            self.touch(k);
            return;
        }
        if self.vals.len() == self.cap {
            let (&fk, &victim) = self.by_freq.iter().next().unwrap();
            self.by_freq.remove(&fk);
            self.vals.remove(&victim);
        }
        self.tick += 1;
        self.vals.insert(k, (v, 1, self.tick));
        self.by_freq.insert((1, self.tick), k);
    }
}

struct XorShift(u64);
impl XorShift {
    fn next(&mut self) -> u64 {
        let mut x = self.0;
        x ^= x << 13;
        x ^= x >> 7;
        x ^= x << 17;
        self.0 = x;
        x
    }
    // skewed distribution: small keys much more likely
    fn zipfish(&mut self, n: u64) -> u64 {
        let a = self.next() % n;
        let b = self.next() % n;
        let c = self.next() % n;
        a.min(b).min(c)
    }
}

fn expensive(k: u64) -> u64 {
    // pretend to be costly: collatz steps
    let mut n = k + 1;
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

fn main() {
    let mut c: Lru<&str, i32> = Lru::new(3);
    c.put("a", 1);
    c.put("b", 2);
    c.put("c", 3);
    println!("{:?}", c.keys_mru());
    println!("get a = {:?}", c.get(&"a"));
    c.put("d", 4);
    println!("after d: {:?}, evicted {:?}", c.keys_mru(), c.evictions);
    println!("put b again -> {:?}", c.put("b", 20));
    println!("get c = {:?}", c.get(&"c"));
    if let Some(v) = c.get_mut(&"a") {
        *v *= 100;
    }
    let a = c.get(&"a").copied();
    println!("{:?} a={:?}", c.keys_mru(), a);
    println!("remove d: {} remove z: {}", c.remove(&"d"), c.remove(&"z"));
    c.put("e", 5);
    c.put("f", 6);
    println!("{:?} len={} evicted={:?} slab={} free={}", c.keys_mru(), c.len(), c.evictions, c.slab.len(), c.free.len());
    println!("hits={} misses={} rate={:.3}", c.hits, c.misses, c.hit_rate());

    let mut r = RcLru::new(4);
    for k in [1, 2, 3, 4] {
        r.put(k, format!("v{}", k));
    }
    println!("rc order {:?} rev {:?}", r.order(), r.order_rev());
    println!("get 2 -> {:?}", r.get(2));
    println!("get 9 -> {:?}", r.get(9));
    r.put(5, "v5".into());
    r.put(3, "v3b".into());
    println!("rc order {:?} rev {:?} get3={:?}", r.order(), r.order_rev(), r.get(3));
    for k in 10..20 {
        r.put(k, k.to_string());
        if k % 3 == 0 {
            r.get(k - 2);
        }
    }
    println!("rc order {:?} rev {:?}", r.order(), r.order_rev());
    let strong: Vec<usize> = r.order().iter().map(|k| Rc::strong_count(&r.map[k])).collect();
    let weak: Vec<usize> = r.order().iter().map(|k| Rc::weak_count(&r.map[k])).collect();
    println!("strong counts {:?} weak counts {:?}", strong, weak);

    // workload comparison
    println!("{:>6} {:>10} {:>10} {:>10}", "cap", "lru-hit%", "lfu-hit%", "checksum");
    for cap in [4usize, 16, 64, 256] {
        let mut rng = XorShift(0xDEADBEEF);
        let mut lru: Lru<u64, u64> = Lru::new(cap);
        let mut lfu = Lfu::new(cap);
        let mut checksum = 0u64;
        for i in 0..20_000u64 {
            let k = if i % 1000 < 50 { 5000 + i % 1000 } else { rng.zipfish(1024) };
            let v = match lru.get(&k) {
                Some(&v) => v,
                None => {
                    let v = expensive(k);
                    lru.put(k, v);
                    v
                }
            };
            if lfu.get(k).is_none() {
                lfu.put(k, v);
            }
            checksum = checksum.wrapping_mul(31).wrapping_add(v);
        }
        println!("{:>6} {:>9.2}% {:>9.2}% {:>10}", cap, lru.hit_rate() * 100.0, lfu.hits as f64 * 100.0 / (lfu.hits + lfu.misses) as f64, checksum % 1_000_000_007);
        let mut ev: HashMap<u64, u32> = HashMap::new();
        for k in &lru.evictions {
            *ev.entry(*k).or_default() += 1;
        }
        let mut top: Vec<(u64, u32)> = ev.into_iter().collect();
        top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
        top.truncate(3);
        println!("        most evicted: {:?}, mru head: {:?}", top, lru.keys_mru().into_iter().take(4).collect::<Vec<_>>());
    }
}
