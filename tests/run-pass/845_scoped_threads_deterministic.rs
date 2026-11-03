use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Mutex, RwLock};
use std::thread;

fn is_prime(n: u64) -> bool { if n < 2 { return false; } let mut i = 2; while i * i <= n { if n % i == 0 { return false; } i += 1; } true }

fn main() {
    let data: Vec<u64> = (1..=20_000).collect();
    let chunks: Vec<&[u64]> = data.chunks(3000).collect();
    let partials: Vec<(usize, u64, usize)> = thread::scope(|s| {
        let handles: Vec<_> = chunks.iter().enumerate().map(|(i, c)| s.spawn(move || (i, c.iter().sum::<u64>(), c.iter().filter(|&&x| is_prime(x)).count()))).collect();
        handles.into_iter().map(|h| h.join().unwrap()).collect()
    });
    for p in &partials { println!("chunk {} sum {} primes {}", p.0, p.1, p.2); }
    println!("total {} primes {}", partials.iter().map(|p| p.1).sum::<u64>(), partials.iter().map(|p| p.2).sum::<usize>());

    let counter = Arc::new(AtomicU64::new(0));
    let hits = Arc::new(Mutex::new(Vec::new()));
    let handles: Vec<_> = (0..6).map(|id| {
        let counter = Arc::clone(&counter);
        let hits = Arc::clone(&hits);
        thread::spawn(move || {
            for k in 0..1000 { counter.fetch_add(k % 7, Ordering::Relaxed); }
            hits.lock().unwrap().push(id * id);
            id * 10
        })
    }).collect();
    let rets: Vec<u32> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    let mut hv = hits.lock().unwrap().clone();
    hv.sort();
    println!("counter={} rets={:?} hits={:?}", counter.load(Ordering::SeqCst), rets, hv);

    let (tx, rx) = mpsc::channel::<(usize, String)>();
    thread::scope(|s| {
        for id in 0..4 {
            let tx = tx.clone();
            s.spawn(move || { for j in 0..3 { tx.send((id, format!("w{}-{}", id, j))).unwrap(); } });
        }
    });
    drop(tx);
    let mut msgs: Vec<(usize, String)> = rx.iter().collect();
    msgs.sort();
    println!("{} msgs first={:?} last={:?}", msgs.len(), msgs.first(), msgs.last());

    let table = RwLock::new(vec![0u32; 8]);
    let next = AtomicUsize::new(0);
    thread::scope(|s| {
        for _ in 0..4 {
            s.spawn(|| loop {
                let i = next.fetch_add(1, Ordering::SeqCst);
                if i >= 8 { break; }
                let v = (i as u32 + 1).pow(3);
                table.write().unwrap()[i] = v;
            });
        }
    });
    let t = table.read().unwrap();
    println!("table {:?} sum {}", *t, t.iter().sum::<u32>());

    let mut grid = vec![vec![0u64; 6]; 6];
    thread::scope(|s| {
        for (r, row) in grid.iter_mut().enumerate() {
            s.spawn(move || for (c, cell) in row.iter_mut().enumerate() { *cell = (r as u64 + 1) * (c as u64 + 1); });
        }
    });
    for row in &grid { println!("{}", row.iter().map(|x| format!("{:>3}", x)).collect::<String>()); }
    let r = thread::spawn(|| { if 2 + 2 == 4 { panic!("worker failed") } 5 }).join();
    println!("join err? {}", r.is_err());
    let name = thread::Builder::new().name("named-worker".into()).spawn(|| thread::current().name().map(String::from)).unwrap().join().unwrap();
    println!("{:?}", name);
}
