// LRU cache built on an Rc<RefCell<Node>> doubly linked list with Weak back
// pointers and a HashMap index; checks strong/weak counts and eviction order.
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::{Rc, Weak};

type Link<K, V> = Option<Rc<RefCell<Node<K, V>>>>;

struct Node<K, V> {
    key: K,
    val: V,
    next: Link<K, V>,
    prev: Option<Weak<RefCell<Node<K, V>>>>,
}

impl<K, V> Drop for Node<K, V> {
    fn drop(&mut self) {
        DROPS.with(|d| *d.borrow_mut() += 1);
    }
}

thread_local! { static DROPS: RefCell<u32> = RefCell::new(0); }

struct Lru<K, V> {
    cap: usize,
    map: HashMap<K, Rc<RefCell<Node<K, V>>>>,
    head: Link<K, V>,
    tail: Link<K, V>,
    hits: u32,
    misses: u32,
}

impl<K: std::hash::Hash + Eq + Clone + std::fmt::Debug, V: Clone + std::fmt::Debug> Lru<K, V> {
    fn new(cap: usize) -> Self { Lru { cap, map: HashMap::new(), head: None, tail: None, hits: 0, misses: 0 } }

    fn unlink(&mut self, n: &Rc<RefCell<Node<K, V>>>) {
        let prev = n.borrow_mut().prev.take().and_then(|w| w.upgrade());
        let next = n.borrow_mut().next.take();
        match &prev {
            Some(p) => p.borrow_mut().next = next.clone(),
            None => self.head = next.clone(),
        }
        match &next {
            Some(nx) => nx.borrow_mut().prev = prev.as_ref().map(Rc::downgrade),
            None => self.tail = prev.clone(),
        }
    }

    fn push_front(&mut self, n: Rc<RefCell<Node<K, V>>>) {
        match self.head.take() {
            Some(old) => {
                old.borrow_mut().prev = Some(Rc::downgrade(&n));
                n.borrow_mut().next = Some(old);
            }
            None => self.tail = Some(n.clone()),
        }
        self.head = Some(n);
    }

    fn get(&mut self, k: &K) -> Option<V> {
        let n = match self.map.get(k) {
            Some(n) => n.clone(),
            None => { self.misses += 1; return None; }
        };
        self.hits += 1;
        self.unlink(&n);
        let v = n.borrow().val.clone();
        self.push_front(n);
        Some(v)
    }

    fn put(&mut self, k: K, v: V) -> Option<(K, V)> {
        if let Some(n) = self.map.get(&k).cloned() {
            n.borrow_mut().val = v;
            self.unlink(&n);
            self.push_front(n);
            return None;
        }
        let mut evicted = None;
        if self.map.len() == self.cap {
            if let Some(t) = self.tail.clone() {
                self.unlink(&t);
                let key = t.borrow().key.clone();
                self.map.remove(&key);
                let val = t.borrow().val.clone();
                evicted = Some((key, val));
            }
        }
        let n = Rc::new(RefCell::new(Node { key: k.clone(), val: v, next: None, prev: None }));
        self.map.insert(k, n.clone());
        self.push_front(n);
        evicted
    }

    fn order(&self) -> Vec<K> {
        let mut out = vec![];
        let mut cur = self.head.clone();
        while let Some(n) = cur {
            out.push(n.borrow().key.clone());
            cur = n.borrow().next.clone();
        }
        out
    }
    fn order_rev(&self) -> Vec<K> {
        let mut out = vec![];
        let mut cur = self.tail.clone();
        while let Some(n) = cur {
            out.push(n.borrow().key.clone());
            cur = n.borrow().prev.as_ref().and_then(Weak::upgrade);
        }
        out
    }
    fn counts(&self, k: &K) -> Option<(usize, usize)> {
        self.map.get(k).map(|n| (Rc::strong_count(n), Rc::weak_count(n)))
    }
}

impl<K, V> Drop for Lru<K, V> {
    fn drop(&mut self) {
        // break the forward chain iteratively to avoid deep recursion
        let mut cur = self.head.take();
        while let Some(n) = cur {
            cur = n.borrow_mut().next.take();
        }
    }
}

fn main() {
    {
        let mut c: Lru<String, u32> = Lru::new(3);
        for (i, w) in ["a", "b", "c"].iter().enumerate() {
            c.put(w.to_string(), i as u32);
        }
        println!("order {:?} rev {:?}", c.order(), c.order_rev());
        println!("get b = {:?}", c.get(&"b".to_string()));
        println!("order {:?}", c.order());
        println!("put d evicts {:?}", c.put("d".into(), 10));
        println!("put b (update) evicts {:?}", c.put("b".into(), 99));
        println!("order {:?} rev {:?}", c.order(), c.order_rev());
        println!("get a = {:?} get c = {:?}", c.get(&"a".into()), c.get(&"c".into()));
        for k in ["x", "y", "b", "z"] {
            let ev = c.put(k.to_string(), k.len() as u32 * 7);
            println!("put {} evicts {:?} -> {:?}", k, ev, c.order());
        }
        let head = c.order()[0].clone();
        let tail = c.order_rev()[0].clone();
        let mid = c.order()[1].clone();
        println!("counts head {:?} mid {:?} tail {:?}", c.counts(&head), c.counts(&mid), c.counts(&tail));
        println!("hits={} misses={} drops so far={}", c.hits, c.misses, DROPS.with(|d| *d.borrow()));
    }
    println!("drops after cache dropped={}", DROPS.with(|d| *d.borrow()));

    let mut big: Lru<u32, u64> = Lru::new(50);
    let mut x: u64 = 12345;
    let mut evictions = 0;
    for _ in 0..2000 {
        x = x.wrapping_mul(6364136223846793005).wrapping_add(1442695040888963407);
        let k = ((x >> 33) % 120) as u32;
        if big.get(&k).is_none() && big.put(k, x >> 40).is_some() {
            evictions += 1;
        }
    }
    let o = big.order();
    println!("big: len={} evictions={} hits={} misses={} head5={:?}", o.len(), evictions, big.hits, big.misses, &o[..5]);
    let rev = big.order_rev();
    println!("consistent: {}", rev.iter().rev().eq(o.iter()));
    drop(big);
    println!("total drops={}", DROPS.with(|d| *d.borrow()));
}
