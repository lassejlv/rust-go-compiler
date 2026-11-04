// thread::scope borrowing locals, spawn+join returning values, Builder names.
use std::sync::atomic::{AtomicU64, AtomicUsize, Ordering};
use std::sync::Mutex;
use std::thread;

fn parallel_sum(data: &[u64], chunks: usize) -> u64 {
    let total = AtomicU64::new(0);
    thread::scope(|s| {
        for c in data.chunks(data.len().div_ceil(chunks)) {
            let total = &total;
            s.spawn(move || {
                let part: u64 = c.iter().sum();
                total.fetch_add(part, Ordering::Relaxed);
            });
        }
    });
    total.into_inner()
}

fn main() {
    let data: Vec<u64> = (1..=1000).collect();
    println!("sum={}", parallel_sum(&data, 7));

    // scoped threads returning values via ScopedJoinHandle
    let words = vec!["apple", "banana", "cherry", "date"];
    let lens: Vec<usize> = thread::scope(|s| {
        let hs: Vec<_> = words.iter().map(|w| s.spawn(move || w.len())).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!("lens={:?}", lens);

    // mutate disjoint parts of a vector from scoped threads
    let mut grid = vec![0u32; 12];
    thread::scope(|s| {
        for (i, row) in grid.chunks_mut(4).enumerate() {
            s.spawn(move || {
                for (j, x) in row.iter_mut().enumerate() {
                    *x = (i * 10 + j) as u32;
                }
            });
        }
    });
    println!("grid={:?}", grid);

    // non-scoped spawn returning owned values
    let handles: Vec<thread::JoinHandle<(usize, String)>> = (0..5)
        .map(|i| {
            thread::Builder::new()
                .name(format!("worker-{}", i))
                .spawn(move || {
                    let name = thread::current().name().unwrap().to_string();
                    (i * i, name)
                })
                .unwrap()
        })
        .collect();
    let mut results: Vec<_> = handles.into_iter().map(|h| h.join().unwrap()).collect();
    results.sort();
    for r in &results {
        println!("{:?}", r);
    }

    // shared mutex inside scope + nested scope spawns
    let log = Mutex::new(Vec::new());
    let counter = AtomicUsize::new(0);
    thread::scope(|s| {
        for i in 0..3 {
            let log = &log;
            let counter = &counter;
            s.spawn(move || {
                thread::scope(|s2| {
                    for j in 0..2 {
                        s2.spawn(move || {
                            counter.fetch_add(1, Ordering::SeqCst);
                            log.lock().unwrap().push(i * 10 + j);
                        });
                    }
                });
            });
        }
    });
    let mut l = log.into_inner().unwrap();
    l.sort();
    println!("log={:?} count={}", l, counter.load(Ordering::SeqCst));
    let h = thread::spawn(|| thread::current().name().map(|s| s.to_string()));
    println!("unnamed: {:?}", h.join().unwrap());
    println!("main name: {:?}", thread::current().name());
    let r = thread::scope(|s| {
        let h = s.spawn(|| 40);
        h.join().unwrap() + 2
    });
    println!("r={}", r);
}
