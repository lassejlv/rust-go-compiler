use std::collections::{HashMap, VecDeque};
use std::fmt::Debug;
use std::hash::Hash;

struct LruCache<K, V> {
    cap: usize,
    map: HashMap<K, V>,
    order: VecDeque<K>,
    hits: u32,
    misses: u32,
    evictions: Vec<K>,
}

impl<K: Hash + Eq + Clone + Debug, V: Clone + Debug> LruCache<K, V> {
    fn new(cap: usize) -> Self {
        LruCache { cap, map: HashMap::new(), order: VecDeque::new(), hits: 0, misses: 0, evictions: Vec::new() }
    }

    fn touch(&mut self, k: &K) {
        if let Some(pos) = self.order.iter().position(|x| x == k) {
            let key = self.order.remove(pos).unwrap();
            self.order.push_back(key);
        }
    }

    fn get(&mut self, k: &K) -> Option<V> {
        if self.map.contains_key(k) {
            self.hits += 1;
            self.touch(k);
            self.map.get(k).cloned()
        } else {
            self.misses += 1;
            None
        }
    }

    fn put(&mut self, k: K, v: V) -> Option<V> {
        if let Some(old) = self.map.insert(k.clone(), v) {
            self.touch(&k);
            return Some(old);
        }
        self.order.push_back(k);
        if self.order.len() > self.cap {
            if let Some(evicted) = self.order.pop_front() {
                self.map.remove(&evicted);
                self.evictions.push(evicted);
            }
        }
        None
    }

    fn get_or_insert_with<F: FnOnce() -> V>(&mut self, k: K, f: F) -> V {
        match self.get(&k) {
            Some(v) => v,
            None => {
                let v = f();
                self.put(k, v.clone());
                v
            }
        }
    }

    fn keys_lru_to_mru(&self) -> Vec<K> {
        self.order.iter().cloned().collect()
    }
}

fn slow_square(n: u64, calls: &mut u32) -> u64 {
    *calls += 1;
    n * n
}

fn main() {
    let mut c: LruCache<&str, i32> = LruCache::new(3);
    c.put("a", 1);
    c.put("b", 2);
    c.put("c", 3);
    println!("{:?}", c.keys_lru_to_mru());
    println!("get a {:?}", c.get(&"a"));
    println!("{:?}", c.keys_lru_to_mru());
    c.put("d", 4);
    println!("after d {:?} evicted {:?}", c.keys_lru_to_mru(), c.evictions);
    println!("get b {:?}", c.get(&"b"));
    println!("replace c -> {:?}", c.put("c", 30));
    println!("{:?}", c.keys_lru_to_mru());
    c.put("e", 5);
    c.put("f", 6);
    println!("{:?} evicted {:?}", c.keys_lru_to_mru(), c.evictions);
    println!("hits {} misses {}", c.hits, c.misses);

    let mut cache: LruCache<u64, u64> = LruCache::new(4);
    let mut calls = 0;
    let requests = [1, 2, 3, 1, 4, 5, 1, 2, 6, 1, 3, 3];
    let mut results = Vec::new();
    for &r in requests.iter() {
        let v = cache.get_or_insert_with(r, || slow_square(r, &mut calls));
        results.push(v);
    }
    println!("{:?}", results);
    println!("computed {} times, hits {}, misses {}", calls, cache.hits, cache.misses);
    println!("final order {:?}", cache.keys_lru_to_mru());
    println!("evictions {:?}", cache.evictions);

    let mut sc: LruCache<String, Vec<String>> = LruCache::new(2);
    for word in "apple avocado banana blueberry cherry apple".split(' ') {
        let key = word[..1].to_string();
        let mut list = sc.get(&key).unwrap_or_default();
        list.push(word.to_string());
        sc.put(key, list);
    }
    for k in sc.keys_lru_to_mru() {
        println!("{} -> {:?}", k, sc.map[&k]);
    }
    println!("evicted {:?}", sc.evictions);
}
