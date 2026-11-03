// Deterministic parallelism: scoped threads over chunks, mpsc fan-in with
// results sorted by key, Arc<Mutex<BTreeMap>> aggregation, atomics, a
// barrier-synchronized two-phase computation, and thread panics joined as Err.
use std::collections::BTreeMap;
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::{mpsc, Arc, Barrier, Mutex, RwLock};
use std::thread;

fn is_prime(n: u64) -> bool { n >= 2 && (2..).take_while(|d| d * d <= n).all(|d| n % d != 0) }

fn main() {
    let data: Vec<u64> = (1..=20_000).collect();
    let chunk_sums: Vec<(usize, u64, usize)> = thread::scope(|s| {
        let handles: Vec<_> = data.chunks(3000).enumerate().map(|(i, ch)| {
            s.spawn(move || (i, ch.iter().sum::<u64>(), ch.iter().filter(|&&x| is_prime(x)).count()))
        }).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!("chunks: {:?}", chunk_sums);
    let total: u64 = chunk_sums.iter().map(|c| c.1).sum();
    let primes: usize = chunk_sums.iter().map(|c| c.2).sum();
    println!("total {} primes {}", total, primes);

    let text = "the cat sat on the mat the end a cat a hat on a mat";
    let words: Vec<&str> = text.split(' ').collect();
    let freq = Arc::new(Mutex::new(BTreeMap::<String, usize>::new()));
    thread::scope(|s| {
        for part in words.chunks(4) {
            let freq = Arc::clone(&freq);
            s.spawn(move || {
                let mut local = BTreeMap::new();
                for w in part { *local.entry(w.to_string()).or_insert(0) += 1; }
                let mut g = freq.lock().unwrap();
                for (k, v) in local { *g.entry(k).or_insert(0) += v; }
            });
        }
    });
    println!("freq: {:?}", freq.lock().unwrap());

    let (tx, rx) = mpsc::channel::<(u32, String)>();
    let mut joins = vec![];
    for id in 0..4u32 {
        let tx = tx.clone();
        joins.push(thread::spawn(move || {
            for k in 0..3 { tx.send((id * 10 + k, format!("w{}-{}", id, k * k))).unwrap(); }
            id * 100
        }));
    }
    drop(tx);
    let mut got: Vec<(u32, String)> = rx.iter().collect();
    got.sort();
    let rets: Vec<u32> = joins.into_iter().map(|j| j.join().unwrap()).collect();
    println!("messages {} first {:?} last {:?} rets {:?}", got.len(), got.first(), got.last(), rets);

    let counter = Arc::new(AtomicUsize::new(0));
    let maxv = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..8u64).map(|i| {
        let (c, m) = (counter.clone(), maxv.clone());
        thread::spawn(move || {
            for j in 0..1000 { c.fetch_add(1, Ordering::Relaxed); m.fetch_max(i * 1000 + j, Ordering::Relaxed); }
        })
    }).collect();
    for h in hs { h.join().unwrap(); }
    println!("atomic count {} max {}", counter.load(Ordering::SeqCst), maxv.load(Ordering::SeqCst));

    let n = 4;
    let barrier = Arc::new(Barrier::new(n));
    let cells = Arc::new(RwLock::new(vec![1u64; n]));
    let hs: Vec<_> = (0..n).map(|i| {
        let (b, c) = (barrier.clone(), cells.clone());
        thread::spawn(move || {
            for _round in 0..3 {
                let next = { let r = c.read().unwrap(); r[i] + r[(i + 1) % n] * 2 };
                b.wait();
                c.write().unwrap()[i] = next;
                b.wait();
            }
        })
    }).collect();
    for h in hs { h.join().unwrap(); }
    println!("barrier rounds result {:?}", cells.read().unwrap());

    std::panic::set_hook(Box::new(|_| {}));
    let bad = thread::spawn(|| { let v: Vec<i32> = vec![]; v[3] });
    let r = bad.join();
    let msg = r.as_ref().err().and_then(|e| e.downcast_ref::<String>().cloned()).unwrap_or_default();
    println!("thread panic joined: is_err={} msg={:?}", r.is_err(), msg);
    let res = thread::scope(|s| {
        let a = s.spawn(|| 1);
        let b = s.spawn(|| -> i32 { panic!("scoped boom") });
        (a.join(), b.join().is_err())
    });
    println!("scoped: {:?}", (res.0.ok(), res.1));
    let named = thread::Builder::new().name("worker-x".into()).spawn(|| thread::current().name().map(String::from)).unwrap().join().unwrap();
    println!("named thread {:?}", named);
}
