// Arc<Mutex>, Arc<RwLock>, threads joined in order, deterministic aggregation, Barrier, Condvar handoff
use std::collections::BTreeMap;
use std::sync::{Arc, Barrier, Condvar, Mutex, RwLock};
use std::thread;

fn parallel_sum(data: Vec<u64>, n: usize) -> u64 {
    let data = Arc::new(data);
    let chunk = (data.len() + n - 1) / n;
    let handles: Vec<_> = (0..n)
        .map(|i| {
            let d = Arc::clone(&data);
            thread::spawn(move || {
                let lo = (i * chunk).min(d.len());
                let hi = ((i + 1) * chunk).min(d.len());
                d[lo..hi].iter().sum::<u64>()
            })
        })
        .collect();
    handles.into_iter().map(|h| h.join().unwrap()).sum()
}

fn main() {
    println!("{}", parallel_sum((1..=10_000).collect(), 7));
    let counter = Arc::new(Mutex::new(0u64));
    let hist = Arc::new(Mutex::new(BTreeMap::new()));
    let mut hs = vec![];
    for t in 0..8 {
        let c = Arc::clone(&counter);
        let h = Arc::clone(&hist);
        hs.push(thread::spawn(move || {
            for i in 0..1000 {
                *c.lock().unwrap() += 1;
                if i % 250 == 0 {
                    *h.lock().unwrap().entry(t % 3).or_insert(0) += 1;
                }
            }
            t * 10
        }));
    }
    let rets: Vec<u32> = hs.into_iter().map(|h| h.join().unwrap()).collect();
    println!("{:?}", rets);
    println!("counter {}", *counter.lock().unwrap());
    println!("hist {:?}", hist.lock().unwrap());
    println!("arc strong {}", Arc::strong_count(&counter));

    let cfg = Arc::new(RwLock::new(vec![1, 2, 3]));
    let readers: Vec<_> = (0..4)
        .map(|i| {
            let c = cfg.clone();
            thread::spawn(move || c.read().unwrap().iter().map(|x| x * i).sum::<i32>())
        })
        .collect();
    let sums: Vec<i32> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("{:?}", sums);
    cfg.write().unwrap().push(4);
    println!("{:?}", *cfg.read().unwrap());

    let barrier = Arc::new(Barrier::new(4));
    let order = Arc::new(Mutex::new(Vec::new()));
    let hs: Vec<_> = (0..4)
        .map(|i| {
            let b = barrier.clone();
            let o = order.clone();
            thread::spawn(move || {
                o.lock().unwrap().push(format!("before-{}", i));
                let leader = b.wait().is_leader();
                o.lock().unwrap().push(format!("after-{}", i));
                leader
            })
        })
        .collect();
    let leaders = hs.into_iter().map(|h| h.join().unwrap()).filter(|&l| l).count();
    let log = order.lock().unwrap();
    let first_after = log.iter().position(|s| s.starts_with("after")).unwrap();
    println!("leaders={} all befores first={}", leaders, first_after == 4);

    let pair = Arc::new((Mutex::new(Vec::<u32>::new()), Condvar::new()));
    let p2 = pair.clone();
    let producer = thread::spawn(move || {
        for i in 1..=5 {
            let (m, cv) = &*p2;
            m.lock().unwrap().push(i * i);
            cv.notify_one();
        }
    });
    let mut got = vec![];
    {
        let (m, cv) = &*pair;
        let mut q = m.lock().unwrap();
        while got.len() < 5 {
            q = cv.wait_while(q, |q| q.is_empty()).unwrap();
            got.extend(q.drain(..));
        }
    }
    producer.join().unwrap();
    println!("{:?}", got);

    let m = Mutex::new(5);
    {
        let mut g = m.lock().unwrap();
        *g += 1;
        println!("try_lock while held: {}", m.try_lock().is_err());
    }
    println!("{:?}", m.into_inner().unwrap());
    let poisoned = Arc::new(Mutex::new(1));
    let p = poisoned.clone();
    let _ = thread::spawn(move || {
        let _g = p.lock().unwrap();
        panic!("poison");
    })
    .join();
    println!("poisoned={}", poisoned.is_poisoned());
    let v = match poisoned.lock() {
        Ok(g) => *g,
        Err(e) => *e.into_inner() + 100,
    };
    println!("{}", v);
    let results: Vec<String> = thread::scope(|s| {
        let hs: Vec<_> = ["a", "bb", "ccc"].iter().map(|w| s.spawn(move || w.repeat(2))).collect();
        hs.into_iter().map(|h| h.join().unwrap()).collect()
    });
    println!("{:?}", results);
    let name = thread::Builder::new().name("worker-7".into()).spawn(|| thread::current().name().map(String::from)).unwrap().join().unwrap();
    println!("{:?}", name);
}
