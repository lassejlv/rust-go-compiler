// atomics: fetch_add/sub/and/or/xor/max/min, compare_exchange loops, fetch_update, AtomicBool flags, statics, spin lock
use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicU64, AtomicUsize, Ordering::*};
use std::sync::Arc;
use std::thread;

static GLOBAL_ID: AtomicUsize = AtomicUsize::new(1);

fn next_id() -> usize {
    GLOBAL_ID.fetch_add(1, Relaxed)
}

struct SpinLock {
    locked: AtomicBool,
}
impl SpinLock {
    fn lock(&self) {
        while self.locked.compare_exchange_weak(false, true, Acquire, Relaxed).is_err() {
            std::hint::spin_loop();
        }
    }
    fn unlock(&self) {
        self.locked.store(false, Release);
    }
}

fn main() {
    let a = AtomicU32::new(10);
    println!("{} {}", a.fetch_add(5, SeqCst), a.load(SeqCst));
    println!("{} {}", a.fetch_sub(3, SeqCst), a.load(SeqCst));
    println!("{} {:#b}", a.fetch_and(0b1010, SeqCst), a.load(SeqCst));
    println!("{} {:#b}", a.fetch_or(0b0101, SeqCst), a.load(SeqCst));
    println!("{} {:#b}", a.fetch_xor(0b1111, SeqCst), a.load(SeqCst));
    println!("{} {}", a.fetch_max(100, SeqCst), a.fetch_min(7, SeqCst));
    println!("{}", a.load(SeqCst));
    println!("{}", a.swap(42, SeqCst));
    println!("{:?}", a.compare_exchange(42, 43, SeqCst, SeqCst));
    println!("{:?}", a.compare_exchange(42, 44, SeqCst, SeqCst));
    println!("{:?}", a.fetch_update(SeqCst, SeqCst, |x| if x < 50 { Some(x * 2) } else { None }));
    println!("{:?}", a.fetch_update(SeqCst, SeqCst, |x| if x < 50 { Some(x * 2) } else { None }));
    println!("{}", a.into_inner());
    let w = AtomicU32::new(u32::MAX);
    w.fetch_add(2, Relaxed);
    println!("wrapped {}", w.load(Relaxed));
    let s = AtomicI64::new(-5);
    s.fetch_sub(i64::MAX, Relaxed);
    println!("{}", s.load(Relaxed));
    let flag = AtomicBool::new(false);
    println!("{} {}", flag.fetch_or(true, SeqCst), flag.fetch_and(false, SeqCst));
    println!("{} {}", flag.fetch_xor(true, SeqCst), flag.load(SeqCst));
    println!("{}", flag.fetch_nand(true, SeqCst));
    println!("{}", flag.load(SeqCst));
    let ids: Vec<usize> = (0..3).map(|_| next_id()).collect();
    println!("{:?}", ids);
    let counter = Arc::new(AtomicU64::new(0));
    let maxv = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..6u64)
        .map(|t| {
            let c = counter.clone();
            let m = maxv.clone();
            thread::spawn(move || {
                for i in 0..5000u64 {
                    c.fetch_add(1, Relaxed);
                    let v = t * 10000 + i;
                    let mut cur = m.load(Relaxed);
                    while v > cur {
                        match m.compare_exchange_weak(cur, v, Relaxed, Relaxed) {
                            Ok(_) => break,
                            Err(x) => cur = x,
                        }
                    }
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("count {} max {}", counter.load(SeqCst), maxv.load(SeqCst));
    let lock = Arc::new(SpinLock { locked: AtomicBool::new(false) });
    let shared = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..4)
        .map(|_| {
            let l = lock.clone();
            let sh = shared.clone();
            thread::spawn(move || {
                for _ in 0..1000 {
                    l.lock();
                    let v = sh.load(Relaxed);
                    sh.store(v + 1, Relaxed);
                    l.unlock();
                }
            })
        })
        .collect();
    hs.into_iter().for_each(|h| h.join().unwrap());
    println!("spin total {}", shared.load(SeqCst));
    let budget = Arc::new(AtomicUsize::new(10));
    let hs: Vec<_> = (0..5)
        .map(|_| {
            let b = budget.clone();
            thread::spawn(move || {
                let mut got = 0;
                while b.fetch_update(SeqCst, SeqCst, |x| x.checked_sub(1)).is_ok() {
                    got += 1;
                }
                got
            })
        })
        .collect();
    let total: usize = hs.into_iter().map(|h| h.join().unwrap()).sum();
    println!("budget consumed {} left {}", total, budget.load(SeqCst));
    let mut local = AtomicUsize::new(3);
    *local.get_mut() += 4;
    println!("{}", local.load(Relaxed));
    let arr = [AtomicU32::new(0), AtomicU32::new(0), AtomicU32::new(0)];
    thread::scope(|s| {
        for i in 0..9 {
            let arr = &arr;
            s.spawn(move || arr[i % 3].fetch_add(i as u32, Relaxed));
        }
    });
    println!("{:?}", arr.iter().map(|a| a.load(Relaxed)).collect::<Vec<_>>());
    std::sync::atomic::fence(SeqCst);
    println!("{:?}", AtomicU32::new(7));
}
