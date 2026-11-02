// LRU cache using an index-linked doubly linked list stored in a Vec plus a HashMap of indices.
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

const NIL: usize = usize::MAX;

struct Node<K, V> {
    key: K,
    val: V,
    prev: usize,
    next: usize,
}

struct Lru<K, V> {
    nodes: Vec<Node<K, V>>,
    map: HashMap<K, usize>,
    head: usize, // most recent
    tail: usize, // least recent
    cap: usize,
    hits: u32,
    misses: u32,
}

impl<K: Hash + Eq + Clone + Debug, V: Debug> Lru<K, V> {
    fn new(cap: usize) -> Self {
        assert!(cap > 0);
        Lru { nodes: Vec::with_capacity(cap), map: HashMap::new(), head: NIL, tail: NIL, cap, hits: 0, misses: 0 }
    }

    fn unlink(&mut self, i: usize) {
        let (p, n) = (self.nodes[i].prev, self.nodes[i].next);
        if p != NIL {
            self.nodes[p].next = n;
        } else {
            self.head = n;
        }
        if n != NIL {
            self.nodes[n].prev = p;
        } else {
            self.tail = p;
        }
    }

    fn push_front(&mut self, i: usize) {
        self.nodes[i].prev = NIL;
        self.nodes[i].next = self.head;
        if self.head != NIL {
            self.nodes[self.head].prev = i;
        }
        self.head = i;
        if self.tail == NIL {
            self.tail = i;
        }
    }

    fn get(&mut self, k: &K) -> Option<&V> {
        match self.map.get(k).copied() {
            Some(i) => {
                self.hits += 1;
                self.unlink(i);
                self.push_front(i);
                Some(&self.nodes[i].val)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }

    fn peek(&self, k: &K) -> Option<&V> {
        self.map.get(k).map(|&i| &self.nodes[i].val)
    }

    fn put(&mut self, k: K, v: V) -> Option<(K, V)> {
        if let Some(&i) = self.map.get(&k) {
            self.nodes[i].val = v;
            self.unlink(i);
            self.push_front(i);
            return None;
        }
        if self.nodes.len() < self.cap {
            let i = self.nodes.len();
            self.nodes.push(Node { key: k.clone(), val: v, prev: NIL, next: NIL });
            self.map.insert(k, i);
            self.push_front(i);
            None
        } else {
            // reuse the tail slot
            let i = self.tail;
            self.unlink(i);
            let old_key = std::mem::replace(&mut self.nodes[i].key, k.clone());
            let old_val = std::mem::replace(&mut self.nodes[i].val, v);
            self.map.remove(&old_key);
            self.map.insert(k, i);
            self.push_front(i);
            Some((old_key, old_val))
        }
    }

    fn order(&self) -> Vec<&K> {
        let mut out = vec![];
        let mut i = self.head;
        while i != NIL {
            out.push(&self.nodes[i].key);
            i = self.nodes[i].next;
        }
        out
    }

    fn order_rev(&self) -> Vec<&K> {
        let mut out = vec![];
        let mut i = self.tail;
        while i != NIL {
            out.push(&self.nodes[i].key);
            i = self.nodes[i].prev;
        }
        out
    }
}

fn fib_cached(n: u64, cache: &mut Lru<u64, u64>) -> u64 {
    if n < 2 {
        return n;
    }
    if let Some(v) = cache.get(&n) {
        return *v;
    }
    let v = fib_cached(n - 1, cache) + fib_cached(n - 2, cache);
    cache.put(n, v);
    v
}

fn main() {
    let mut c: Lru<&str, i32> = Lru::new(3);
    println!("{:?}", c.put("a", 1));
    println!("{:?}", c.put("b", 2));
    println!("{:?}", c.put("c", 3));
    println!("order {:?}", c.order());
    println!("get a = {:?}", c.get(&"a"));
    println!("order {:?}", c.order());
    println!("evicted {:?}", c.put("d", 4));
    println!("order {:?} rev {:?}", c.order(), c.order_rev());
    println!("get b = {:?}", c.get(&"b"));
    println!("update c: {:?}", c.put("c", 33));
    println!("order {:?} peek c={:?}", c.order(), c.peek(&"c"));
    println!("evicted {:?}", c.put("e", 5));
    println!("evicted {:?}", c.put("f", 6));
    println!("order {:?} hits={} misses={}", c.order(), c.hits, c.misses);

    let mut fc: Lru<u64, u64> = Lru::new(4);
    println!("fib(60) = {}", fib_cached(60, &mut fc));
    println!("hits={} misses={} cached={:?}", fc.hits, fc.misses, fc.order());

    let mut sc: Lru<String, Vec<u8>> = Lru::new(2);
    for w in ["alpha", "beta", "alpha", "gamma", "beta", "alpha"] {
        let hit = sc.get(&w.to_string()).is_some();
        if !hit {
            let ev = sc.put(w.to_string(), w.bytes().collect());
            println!("miss {:<6} evicted={:?}", w, ev.map(|(k, v)| (k, v.len())));
        } else {
            println!("hit  {}", w);
        }
    }
}
