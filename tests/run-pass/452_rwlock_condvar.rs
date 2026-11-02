// RwLock readers/writers, Condvar-based producer/consumer with deterministic ordering (strict
// turn-taking), a barrier-like phase sync built from Mutex+Condvar, and std::sync::Barrier.
use std::collections::VecDeque;
use std::sync::{Arc, Barrier, Condvar, Mutex, RwLock};
use std::thread;

struct Turn {
    state: Mutex<(usize, Vec<String>)>, // (whose turn, log)
    cv: Condvar,
}

fn main() {
    // RwLock
    let cfg = Arc::new(RwLock::new(vec![1, 2, 3]));
    {
        let r1 = cfg.read().unwrap();
        let r2 = cfg.read().unwrap();
        println!("two readers {} {} write blocked={}", r1.len(), r2[0], cfg.try_write().is_err());
    }
    {
        let mut w = cfg.write().unwrap();
        w.push(4);
        println!("read blocked during write: {}", cfg.try_read().is_err());
    }
    let readers: Vec<_> = (0..3)
        .map(|i| {
            let c = Arc::clone(&cfg);
            thread::spawn(move || c.read().unwrap().iter().map(|x| x * (i + 1)).sum::<i32>())
        })
        .collect();
    let sums: Vec<i32> = readers.into_iter().map(|h| h.join().unwrap()).collect();
    println!("reader sums {:?}", sums);

    // strict turn taking between 3 threads, 3 rounds each
    let turn = Arc::new(Turn { state: Mutex::new((0, Vec::new())), cv: Condvar::new() });
    let n = 3;
    let hs: Vec<_> = (0..n)
        .map(|id| {
            let t = Arc::clone(&turn);
            thread::spawn(move || {
                for round in 0..3 {
                    let mut g = t.state.lock().unwrap();
                    while g.0 % n != id {
                        g = t.cv.wait(g).unwrap();
                    }
                    g.1.push(format!("t{}r{}", id, round));
                    g.0 += 1;
                    t.cv.notify_all();
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("{}", turn.state.lock().unwrap().1.join(" "));

    // bounded producer/consumer queue with condvars
    let q = Arc::new((Mutex::new((VecDeque::new(), false)), Condvar::new(), Condvar::new()));
    let cap = 2;
    let prod = {
        let q = Arc::clone(&q);
        thread::spawn(move || {
            let (m, not_full, not_empty) = &*q;
            for i in 1..=6 {
                let mut g = m.lock().unwrap();
                while g.0.len() >= cap {
                    g = not_full.wait(g).unwrap();
                }
                g.0.push_back(i * i);
                not_empty.notify_one();
            }
            m.lock().unwrap().1 = true;
            not_empty.notify_all();
        })
    };
    let cons = {
        let q = Arc::clone(&q);
        thread::spawn(move || {
            let (m, not_full, not_empty) = &*q;
            let mut got = Vec::new();
            loop {
                let mut g = m.lock().unwrap();
                while g.0.is_empty() && !g.1 {
                    g = not_empty.wait(g).unwrap();
                }
                match g.0.pop_front() {
                    Some(v) => {
                        got.push(v);
                        not_full.notify_one();
                    }
                    None => break,
                }
            }
            got
        })
    };
    prod.join().unwrap();
    println!("consumed {:?}", cons.join().unwrap());

    // wait_while + timeout-free predicate
    let pair = Arc::new((Mutex::new(0u32), Condvar::new()));
    let p2 = Arc::clone(&pair);
    let h = thread::spawn(move || {
        for _ in 0..5 {
            let (m, c) = &*p2;
            *m.lock().unwrap() += 1;
            c.notify_all();
        }
    });
    {
        let (m, c) = &*pair;
        let g = c.wait_while(m.lock().unwrap(), |v| *v < 5).unwrap();
        println!("waited until {}", *g);
    }
    h.join().unwrap();

    // Barrier: each thread records phase results; output gathered after join and sorted
    let barrier = Arc::new(Barrier::new(4));
    let results = Arc::new(Mutex::new(Vec::new()));
    let hs: Vec<_> = (0..4)
        .map(|i| {
            let b = Arc::clone(&barrier);
            let r = Arc::clone(&results);
            thread::spawn(move || {
                r.lock().unwrap().push((0, i));
                let leader = b.wait().is_leader();
                r.lock().unwrap().push((1, i));
                leader
            })
        })
        .collect();
    let leaders = hs.into_iter().map(|h| h.join().unwrap()).filter(|l| *l).count();
    let mut res = results.lock().unwrap().clone();
    let phase0_before_phase1 = {
        let last0 = res.iter().rposition(|p| p.0 == 0).unwrap();
        let first1 = res.iter().position(|p| p.0 == 1).unwrap();
        last0 < first1
    };
    res.sort();
    println!("leaders={} ordered={} {:?}", leaders, phase0_before_phase1, res);
    let lock = RwLock::new(5);
    *lock.write().unwrap() += 1;
    let rv = *lock.read().unwrap();
    println!("{} {:?}", rv, lock.into_inner());
}
