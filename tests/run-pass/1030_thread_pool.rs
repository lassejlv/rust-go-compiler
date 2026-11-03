use std::sync::mpsc::{self, Receiver, Sender};
use std::sync::{Arc, Mutex};
use std::thread::{self, JoinHandle};

type Job = Box<dyn FnOnce() + Send + 'static>;

struct Worker { id: usize, handle: Option<JoinHandle<usize>> }

struct ThreadPool { workers: Vec<Worker>, sender: Option<Sender<Job>> }

impl ThreadPool {
    fn new(n: usize) -> Self {
        let (tx, rx) = mpsc::channel::<Job>();
        let rx: Arc<Mutex<Receiver<Job>>> = Arc::new(Mutex::new(rx));
        let workers = (0..n).map(|id| {
            let rx = Arc::clone(&rx);
            let handle = thread::Builder::new().name(format!("pool-{}", id)).spawn(move || {
                let mut done = 0;
                loop {
                    let msg = rx.lock().unwrap().recv();
                    match msg {
                        Ok(job) => { job(); done += 1; }
                        Err(_) => break,
                    }
                }
                done
            }).unwrap();
            Worker { id, handle: Some(handle) }
        }).collect();
        ThreadPool { workers, sender: Some(tx) }
    }
    fn execute<F: FnOnce() + Send + 'static>(&self, f: F) {
        self.sender.as_ref().unwrap().send(Box::new(f)).unwrap();
    }
}

impl Drop for ThreadPool {
    fn drop(&mut self) {
        drop(self.sender.take());
        let mut total = 0;
        for w in &mut self.workers {
            if let Some(h) = w.handle.take() {
                total += h.join().unwrap();
                let _ = w.id;
            }
        }
        println!("pool shut down, {} jobs executed by {} workers", total, self.workers.len());
    }
}

fn main() {
    let results = Arc::new(Mutex::new(Vec::new()));
    {
        let pool = ThreadPool::new(4);
        for i in 0..20u64 {
            let r = Arc::clone(&results);
            pool.execute(move || {
                let v = (1..=i).product::<u64>();
                r.lock().unwrap().push((i, v));
            });
        }
    }
    let mut r = results.lock().unwrap().clone();
    r.sort();
    for (i, v) in r.iter().step_by(4) {
        println!("{}! = {}", i, v);
    }
    println!("count {}", r.len());
    let (tx, rx) = mpsc::channel();
    {
        let pool = ThreadPool::new(2);
        for word in ["delta", "alpha", "charlie", "bravo"] {
            let tx = tx.clone();
            pool.execute(move || tx.send(word.to_uppercase()).unwrap());
        }
    }
    drop(tx);
    let mut words: Vec<String> = rx.into_iter().collect();
    words.sort();
    println!("{:?}", words);
}
