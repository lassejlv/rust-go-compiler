// An LRU cache built on a Vec-backed doubly linked list (indices instead of pointers) plus HashMap,
// generic over K: Hash+Eq+Clone and V; get/put/peek/remove/resize, eviction callbacks via
// Box<dyn FnMut>, iteration from most to least recently used, and hit/miss statistics.
use std::collections::HashMap;
use std::fmt::Debug;
use std::hash::Hash;

const NIL: usize = usize::MAX;

struct Entry<K, V> {
    key: K,
    val: V,
    prev: usize,
    next: usize,
}

struct Lru<K, V> {
    map: HashMap<K, usize>,
    slots: Vec<Option<Entry<K, V>>>,
    free: Vec<usize>,
    head: usize,
    tail: usize,
    cap: usize,
    hits: u32,
    misses: u32,
    on_evict: Option<Box<dyn FnMut(&K, &V)>>,
}

impl<K: Hash + Eq + Clone + Debug, V: Debug> Lru<K, V> {
    fn new(cap: usize) -> Self {
        Lru { map: HashMap::new(), slots: vec![], free: vec![], head: NIL, tail: NIL, cap, hits: 0, misses: 0, on_evict: None }
    }
    fn e(&mut self, i: usize) -> &mut Entry<K, V> {
        self.slots[i].as_mut().unwrap()
    }
    fn unlink(&mut self, i: usize) {
        let (p, n) = {
            let e = self.e(i);
            (e.prev, e.next)
        };
        if p != NIL { self.e(p).next = n; } else { self.head = n; }
        if n != NIL { self.e(n).prev = p; } else { self.tail = p; }
    }
    fn push_front(&mut self, i: usize) {
        let h = self.head;
        {
            let e = self.e(i);
            e.prev = NIL;
            e.next = h;
        }
        if h != NIL { self.e(h).prev = i; }
        self.head = i;
        if self.tail == NIL { self.tail = i; }
    }
    fn get(&mut self, k: &K) -> Option<&V> {
        match self.map.get(k).copied() {
            Some(i) => {
                self.hits += 1;
                self.unlink(i);
                self.push_front(i);
                Some(&self.slots[i].as_ref().unwrap().val)
            }
            None => {
                self.misses += 1;
                None
            }
        }
    }
    fn peek(&self, k: &K) -> Option<&V> {
        self.map.get(k).map(|&i| &self.slots[i].as_ref().unwrap().val)
    }
    fn evict_lru(&mut self) {
        let t = self.tail;
        if t == NIL { return; }
        self.unlink(t);
        let e = self.slots[t].take().unwrap();
        self.map.remove(&e.key);
        self.free.push(t);
        if let Some(cb) = self.on_evict.as_mut() { cb(&e.key, &e.val); }
    }
    fn put(&mut self, k: K, v: V) -> Option<V> {
        if let Some(&i) = self.map.get(&k) {
            let old = std::mem::replace(&mut self.e(i).val, v);
            self.unlink(i);
            self.push_front(i);
            return Some(old);
        }
        while self.map.len() >= self.cap { self.evict_lru(); }
        let entry = Entry { key: k.clone(), val: v, prev: NIL, next: NIL };
        let i = match self.free.pop() {
            Some(i) => { self.slots[i] = Some(entry); i }
            None => { self.slots.push(Some(entry)); self.slots.len() - 1 }
        };
        self.map.insert(k, i);
        self.push_front(i);
        None
    }
    fn remove(&mut self, k: &K) -> Option<V> {
        let i = self.map.remove(k)?;
        self.unlink(i);
        self.free.push(i);
        self.slots[i].take().map(|e| e.val)
    }
    fn resize(&mut self, cap: usize) {
        self.cap = cap;
        while self.map.len() > cap { self.evict_lru(); }
    }
    fn order(&self) -> Vec<(K, &V)> {
        let mut out = vec![];
        let mut i = self.head;
        while i != NIL {
            let e = self.slots[i].as_ref().unwrap();
            out.push((e.key.clone(), &e.val));
            i = e.next;
        }
        out
    }
}

fn main() {
    let evicted = std::rc::Rc::new(std::cell::RefCell::new(Vec::new()));
    let ev2 = evicted.clone();
    let mut c: Lru<String, u32> = Lru::new(3);
    c.on_evict = Some(Box::new(move |k: &String, v: &u32| ev2.borrow_mut().push(format!("{}={}", k, v))));
    for (k, v) in [("a", 1), ("b", 2), ("c", 3)] {
        c.put(k.to_string(), v);
    }
    println!("{:?}", c.order());
    println!("get a = {:?}", c.get(&"a".to_string()));
    c.put("d".into(), 4);
    println!("{:?}", c.order());
    let gb = c.get(&"b".to_string()).copied();
    println!("get b = {:?} peek c = {:?}", gb, c.peek(&"c".to_string()));
    println!("replace a -> {:?}", c.put("a".into(), 10));
    c.put("e".into(), 5);
    println!("{:?}", c.order());
    println!("remove d = {:?}, remove zz = {:?}", c.remove(&"d".to_string()), c.remove(&"zz".to_string()));
    c.put("f".into(), 6);
    c.put("g".into(), 7);
    println!("{:?}", c.order());
    c.resize(1);
    println!("{:?} slots={} free={}", c.order(), c.slots.len(), c.free.len());
    println!("evicted {:?}", evicted.borrow());
    println!("hits={} misses={}", c.hits, c.misses);

    // simulate access pattern and measure hit rate for several capacities
    let pattern: Vec<u32> = (0..60).map(|i| (i * 7 + i / 5) % 9).collect();
    for cap in [1, 3, 5, 9] {
        let mut l: Lru<u32, u32> = Lru::new(cap);
        for &p in &pattern {
            if l.get(&p).is_none() { l.put(p, p * p); }
        }
        let keys: Vec<u32> = l.order().into_iter().map(|(k, _)| k).collect();
        println!("cap {} hits {} misses {} mru {:?}", cap, l.hits, l.misses, keys);
    }
}
