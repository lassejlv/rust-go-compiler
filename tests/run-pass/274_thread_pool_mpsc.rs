// A thread pool built from mpsc + Arc<Mutex<Receiver>>, boxed FnOnce jobs, graceful shutdown on
// Drop, result collection via a second channel, and deterministic output by sorting results.
use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

enum Message {
    Run(Job),
    Shutdown,
}

struct Worker {
    id: usize,
    handle: Option<JoinHandle<usize>>,
}

struct ThreadPool {
    workers: Vec<Worker>,
    tx: Sender<Message>,
}

impl ThreadPool {
    fn new(n: usize) -> ThreadPool {
        assert!(n > 0);
        let (tx, rx) = mpsc::channel::<Message>();
        let rx: Arc<Mutex<Receiver<Message>>> = Arc::new(Mutex::new(rx));
        let workers = (0..n)
            .map(|id| {
                let rx = Arc::clone(&rx);
                let handle = thread::Builder::new()
                    .name(format!("worker-{}", id))
                    .spawn(move || {
                        let mut jobs_done = 0;
                        loop {
                            let msg = rx.lock().unwrap().recv();
                            match msg {
                                Ok(Message::Run(job)) => {
                                    job();
                                    jobs_done += 1;
                                }
                                Ok(Message::Shutdown) | Err(_) => break,
                            }
                        }
                        jobs_done
                    })
                    .unwrap();
                Worker { id, handle: Some(handle) }
            })
            .collect();
        ThreadPool { workers, tx }
    }

    fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.tx.send(Message::Run(Box::new(f))).unwrap();
    }

    fn join(mut self) -> usize {
        self.shutdown()
    }

    fn shutdown(&mut self) -> usize {
        for _ in &self.workers {
            let _ = self.tx.send(Message::Shutdown);
        }
        let mut total = 0;
        for w in &mut self.workers {
            if let Some(h) = w.handle.take() {
                let n = h.join().unwrap();
                let _ = w.id;
                total += n;
            }
        }
        total
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        self.shutdown();
    }
}

fn collatz_len(mut n: u64) -> u32 {
    let mut steps = 0;
    while n != 1 {
        n = if n % 2 == 0 { n / 2 } else { 3 * n + 1 };
        steps += 1;
    }
    steps
}

fn main() {
    let pool = ThreadPool::new(4);
    let (rtx, rrx) = mpsc::channel::<(u64, u32)>();
    for n in 1..=40u64 {
        let rtx = rtx.clone();
        pool.execute(move || {
            rtx.send((n, collatz_len(n))).unwrap();
        });
    }
    drop(rtx);
    let mut results: Vec<(u64, u32)> = rrx.iter().collect();
    results.sort();
    println!("got {} results", results.len());
    let best = results.iter().max_by_key(|(n, l)| (*l, std::cmp::Reverse(*n))).unwrap();
    println!("longest chain under 40: n={} len={}", best.0, best.1);
    println!("first 10: {:?}", &results[..10].iter().map(|r| r.1).collect::<Vec<_>>());
    let total_jobs = pool.join();
    println!("total jobs executed: {}", total_jobs);

    // Shared accumulator through Arc<Mutex<>>; pool dropped (not joined) at end of scope
    let acc = Arc::new(Mutex::new(Vec::new()));
    {
        let pool = ThreadPool::new(3);
        for i in 0..12 {
            let acc = Arc::clone(&acc);
            pool.execute(move || {
                let sq = i * i;
                acc.lock().unwrap().push((i, sq));
            });
        }
    }
    let mut v = acc.lock().unwrap().clone();
    v.sort();
    println!("{:?}", v);
    println!("sum of squares = {}", v.iter().map(|p| p.1).sum::<i32>());
    println!("arc strong count = {}", Arc::strong_count(&acc));

    // Jobs that themselves return values through per-job oneshot channels
    let pool = ThreadPool::new(2);
    let receivers: Vec<Receiver<String>> = ["alpha", "beta", "gamma"]
        .iter()
        .map(|w| {
            let (tx, rx) = mpsc::channel();
            let w = w.to_string();
            pool.execute(move || tx.send(w.chars().rev().collect()).unwrap());
            rx
        })
        .collect();
    let outs: Vec<String> = receivers.iter().map(|r| r.recv().unwrap()).collect();
    println!("{:?}", outs);
    let names: Vec<String> = (0..2).map(|i| format!("worker-{}", i)).collect();
    println!("{:?} jobs={}", names, pool.join());
}
