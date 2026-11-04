// A thread pool with a shared job queue (Arc<Mutex<Receiver<Job>>>), graceful shutdown in
// Drop, results gathered through a channel, plus an RwLock-protected cache with stats.
use std::collections::HashMap;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex, RwLock};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Msg {
    Run(Job),
    Stop,
}

struct Pool {
    workers: Vec<(usize, Option<JoinHandle<usize>>)>,
    tx: Sender<Msg>,
}

impl Pool {
    fn new(n: usize) -> Pool {
        let (tx, rx) = mpsc::channel::<Msg>();
        let rx: Arc<Mutex<Receiver<Msg>>> = Arc::new(Mutex::new(rx));
        let workers = (0..n)
            .map(|id| {
                let rx = Arc::clone(&rx);
                let h = thread::spawn(move || {
                    let mut done = 0;
                    loop {
                        let msg = rx.lock().unwrap().recv().unwrap();
                        match msg {
                            Msg::Run(job) => {
                                job();
                                done += 1;
                            }
                            Msg::Stop => break,
                        }
                    }
                    done
                });
                (id, Some(h))
            })
            .collect();
        Pool { workers, tx }
    }
    fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.tx.send(Msg::Run(Box::new(f))).unwrap();
    }
}

impl Drop for Pool {
    fn drop(&mut self) {
        for _ in &self.workers {
            self.tx.send(Msg::Stop).unwrap();
        }
        let mut total = 0;
        for (_, h) in &mut self.workers {
            if let Some(h) = h.take() {
                total += h.join().unwrap();
            }
        }
        println!("pool shut down, {} jobs run by {} workers", total, self.workers.len());
    }
}

struct Cache {
    map: RwLock<HashMap<u64, u64>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
}

impl Cache {
    fn collatz_len(&self, n: u64) -> u64 {
        if let Some(&v) = self.map.read().unwrap().get(&n) {
            self.hits.fetch_add(1, Ordering::Relaxed);
            return v;
        }
        self.misses.fetch_add(1, Ordering::Relaxed);
        let mut steps = 0;
        let mut x = n;
        while x != 1 {
            x = if x % 2 == 0 { x / 2 } else { 3 * x + 1 };
            steps += 1;
        }
        self.map.write().unwrap().entry(n).or_insert(steps);
        steps
    }
}

fn main() {
    let cache = Arc::new(Cache { map: RwLock::new(HashMap::new()), hits: AtomicUsize::new(0), misses: AtomicUsize::new(0) });
    let (rtx, rrx) = mpsc::channel();
    {
        let pool = Pool::new(4);
        for n in 1..=40u64 {
            let cache = cache.clone();
            let rtx = rtx.clone();
            pool.execute(move || {
                let v = cache.collatz_len(n % 20 + 1);
                rtx.send((n, v)).unwrap();
            });
        }
    }
    drop(rtx);
    let mut results: Vec<(u64, u64)> = rrx.iter().collect();
    results.sort();
    println!("{} results", results.len());
    for chunk in results.chunks(10) {
        println!("{:?}", chunk.iter().map(|r| r.1).collect::<Vec<_>>());
    }
    let h = cache.hits.load(Ordering::Relaxed);
    let m = cache.misses.load(Ordering::Relaxed);
    println!("lookups {} entries {}", h + m, cache.map.read().unwrap().len());
    let mut keys: Vec<_> = cache.map.read().unwrap().keys().copied().collect();
    keys.sort_unstable();
    println!("{:?}", keys);
}
