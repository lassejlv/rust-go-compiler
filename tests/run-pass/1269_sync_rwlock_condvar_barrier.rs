// RwLock readers/writers, Condvar wait_while/wait_timeout, Barrier leader election.
use std::sync::{Arc, Barrier, Condvar, Mutex, RwLock};
use std::thread;
use std::time::Duration;

fn main() {
    // RwLock
    let cfg = Arc::new(RwLock::new(vec![1, 2, 3]));
    {
        let r1 = cfg.read().unwrap();
        let r2 = cfg.read().unwrap();
        println!("two readers: {} {}", r1.len(), r2[0]);
        println!("try_write while read: {}", cfg.try_write().is_err());
    }
    let mut hs = vec![];
    for i in 0..4 {
        let cfg = Arc::clone(&cfg);
        hs.push(thread::spawn(move || {
            let mut w = cfg.write().unwrap();
            w.push(10 + i);
        }));
    }
    for h in hs {
        h.join().unwrap();
    }
    let mut v = cfg.read().unwrap().clone();
    v.sort();
    println!("after writers: {:?}", v);
    {
        let w = cfg.write().unwrap();
        println!("try_read while write: {}", cfg.try_read().is_err());
        drop(w);
    }

    // Condvar producer/consumer with wait_while
    let pair = Arc::new((Mutex::new(Vec::<u32>::new()), Condvar::new()));
    let p2 = Arc::clone(&pair);
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
        let mut guard = m.lock().unwrap();
        while got.len() < 5 {
            guard = cv.wait_while(guard, |q| q.is_empty()).unwrap();
            got.extend(guard.drain(..));
        }
    }
    producer.join().unwrap();
    println!("consumed {:?}", got);

    // wait_timeout that times out
    let m = Mutex::new(false);
    let cv = Condvar::new();
    let g = m.lock().unwrap();
    let (g, res) = cv.wait_timeout(g, Duration::from_millis(20)).unwrap();
    println!("timed out: {} flag {}", res.timed_out(), *g);
    drop(g);
    let g = m.lock().unwrap();
    let (g, res) = cv.wait_timeout_while(g, Duration::from_millis(10), |f| !*f).unwrap();
    println!("timeout_while timed out: {} flag {}", res.timed_out(), *g);
    drop(g);

    // Barrier
    let n = 5;
    let barrier = Arc::new(Barrier::new(n));
    let phase = Arc::new(Mutex::new(Vec::new()));
    let hs: Vec<_> = (0..n)
        .map(|i| {
            let b = Arc::clone(&barrier);
            let phase = Arc::clone(&phase);
            thread::spawn(move || {
                phase.lock().unwrap().push(format!("a{}", i));
                let r = b.wait();
                phase.lock().unwrap().push(format!("b{}", i));
                b.wait();
                r.is_leader()
            })
        })
        .collect();
    let leaders = hs.into_iter().map(|h| h.join().unwrap()).filter(|&l| l).count();
    let ph = phase.lock().unwrap();
    let first_b = ph.iter().position(|s| s.starts_with('b')).unwrap();
    println!("leaders={} all a before b: {}", leaders, first_b == n && ph[..n].iter().all(|s| s.starts_with('a')));
    println!("total entries {}", ph.len());
}
