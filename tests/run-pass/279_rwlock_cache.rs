// A read-mostly cache: RwLock<HashMap> with double-checked insertion, Arc<str> values, an
// ArcSwap-like pointer swap via Mutex<Arc<T>> snapshots, and per-key computation counting.
use std::collections::{BTreeMap, HashMap};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

struct Cache {
    map: RwLock<HashMap<u64, Arc<str>>>,
    computes: AtomicUsize,
    hits: AtomicUsize,
}

impl Cache {
    fn get_or_compute(&self, k: u64) -> Arc<str> {
        if let Some(v) = self.map.read().unwrap().get(&k) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Arc::clone(v);
        }
        let mut w = self.map.write().unwrap();
        // another thread may have inserted while we waited for the write lock
        if let Some(v) = w.get(&k) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return Arc::clone(v);
        }
        self.computes.fetch_add(1, Ordering::Relaxed);
        let v: Arc<str> = Arc::from(expensive(k));
        w.insert(k, Arc::clone(&v));
        v
    }
}

fn expensive(k: u64) -> String {
    let mut x = k;
    let mut steps = 0;
    while x != 1 && steps < 1000 {
        x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
        steps += 1;
    }
    format!("k{}:{}", k, steps)
}

// Snapshot-swapping config holder (arc-swap style): readers clone the Arc, writers replace it.
struct Swap<T> {
    cur: Mutex<Arc<T>>,
    version: AtomicUsize,
}

impl<T> Swap<T> {
    fn load(&self) -> Arc<T> {
        Arc::clone(&self.cur.lock().unwrap())
    }
    fn store(&self, v: T) -> Arc<T> {
        self.version.fetch_add(1, Ordering::SeqCst);
        std::mem::replace(&mut *self.cur.lock().unwrap(), Arc::new(v))
    }
    fn rcu(&self, f: impl Fn(&T) -> T) {
        let mut g = self.cur.lock().unwrap();
        let next = f(&g);
        *g = Arc::new(next);
        self.version.fetch_add(1, Ordering::SeqCst);
    }
}

#[derive(Debug, Clone)]
struct Settings {
    font_size: u32,
    plugins: Vec<&'static str>,
}

fn main() {
    let cache = Arc::new(Cache { map: RwLock::new(HashMap::new()), computes: AtomicUsize::new(0), hits: AtomicUsize::new(0) });
    let keys: Vec<u64> = (1..=20).collect();
    let results: Vec<Vec<String>> = thread::scope(|s| {
        (0..6)
            .map(|t| {
                let cache = &cache;
                let keys = &keys;
                s.spawn(move || {
                    keys.iter().cycle().skip(t * 3).take(30).map(|k| cache.get_or_compute(*k).to_string()).collect::<Vec<_>>()
                })
            })
            .collect::<Vec<_>>()
            .into_iter()
            .map(|h| h.join().unwrap())
            .collect()
    });
    let total_calls: usize = results.iter().map(|r| r.len()).sum();
    println!("calls={} computes={} hits={}", total_calls, cache.computes.load(Ordering::SeqCst), cache.hits.load(Ordering::SeqCst));
    println!("computes == distinct keys: {}", cache.computes.load(Ordering::SeqCst) == keys.len());
    let snapshot: BTreeMap<u64, String> = cache.map.read().unwrap().iter().map(|(k, v)| (*k, v.to_string())).collect();
    println!("{:?}", snapshot.values().take(8).collect::<Vec<_>>());
    println!("thread 0 first five: {:?}", &results[0][..5]);
    let v7 = cache.get_or_compute(7);
    println!("arc strong count for k7 (map + local): {}", Arc::strong_count(&v7));

    let cfg = Arc::new(Swap { cur: Mutex::new(Arc::new(Settings { font_size: 12, plugins: vec!["git"] })), version: AtomicUsize::new(0) });
    let old_snapshot = cfg.load();
    thread::scope(|s| {
        for i in 0..4 {
            let cfg = &cfg;
            s.spawn(move || {
                cfg.rcu(|c| Settings { font_size: c.font_size + 1, plugins: c.plugins.clone() });
                if i == 0 {
                    // readers always see a consistent snapshot
                    let snap = cfg.load();
                    assert!(snap.font_size >= 13);
                }
            });
        }
    });
    let prev = cfg.store(Settings { font_size: 99, plugins: vec!["git", "lsp"] });
    println!("old snapshot still {:?}", old_snapshot);
    println!("prev before store font_size={}", prev.font_size);
    println!("now {:?} version={}", cfg.load(), cfg.version.load(Ordering::SeqCst));

    let lock = RwLock::new(vec![1, 2, 3]);
    {
        let r1 = lock.read().unwrap();
        let r2 = lock.read().unwrap();
        println!("two readers: {} {} try_write={}", r1.len(), r2[0], lock.try_write().is_ok());
    }
    lock.write().unwrap().push(4);
    println!("{:?} try_read={}", *lock.read().unwrap(), lock.try_read().is_ok());
    let inner = lock.into_inner().unwrap();
    println!("{:?}", inner);
}
