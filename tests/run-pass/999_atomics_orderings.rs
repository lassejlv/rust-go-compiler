use std::sync::atomic::{AtomicBool, AtomicI32, AtomicU64, AtomicUsize, AtomicPtr, Ordering};
use std::sync::Arc;
use std::thread;

static GLOBAL: AtomicUsize = AtomicUsize::new(0);

struct SpinLock { locked: AtomicBool }
impl SpinLock {
    fn lock(&self) {
        while self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            std::hint::spin_loop();
        }
    }
    fn unlock(&self) { self.locked.store(false, Ordering::Release); }
}

fn main() {
    let hs: Vec<_> = (0..8).map(|_| thread::spawn(|| {
        for _ in 0..1000 { GLOBAL.fetch_add(1, Ordering::Relaxed); }
    })).collect();
    for h in hs { h.join().unwrap(); }
    println!("global {}", GLOBAL.load(Ordering::SeqCst));

    let a = AtomicI32::new(10);
    println!("swap {} now {}", a.swap(20, Ordering::AcqRel), a.load(Ordering::Acquire));
    println!("cas ok {:?}", a.compare_exchange(20, 30, Ordering::SeqCst, Ordering::SeqCst));
    println!("cas fail {:?}", a.compare_exchange(20, 40, Ordering::SeqCst, Ordering::SeqCst));
    println!("fetch_sub {} fetch_and {} fetch_or {} fetch_xor {}",
        a.fetch_sub(5, Ordering::SeqCst), a.fetch_and(0b1110, Ordering::SeqCst),
        a.fetch_or(1, Ordering::SeqCst), a.fetch_xor(0xff, Ordering::SeqCst));
    println!("now {}", a.load(Ordering::SeqCst));
    println!("fetch_max {} fetch_min {} -> {}", a.fetch_max(1000, Ordering::SeqCst), a.fetch_min(-7, Ordering::SeqCst), a.load(Ordering::SeqCst));
    let r = a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| if x < 0 { Some(x * x) } else { None });
    println!("fetch_update {:?} -> {}", r, a.load(Ordering::SeqCst));
    let r = a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| if x < 0 { Some(x * x) } else { None });
    println!("fetch_update {:?}", r);
    let mut owned = AtomicU64::new(5);
    *owned.get_mut() += 1;
    println!("into_inner {}", owned.into_inner());

    let lock = Arc::new(SpinLock { locked: AtomicBool::new(false) });
    let shared = Arc::new(AtomicU64::new(0));
    let hs: Vec<_> = (0..4).map(|i| {
        let l = lock.clone();
        let s = shared.clone();
        thread::spawn(move || {
            for _ in 0..250 {
                l.lock();
                let v = s.load(Ordering::Relaxed);
                s.store(v + i, Ordering::Relaxed);
                l.unlock();
            }
        })
    }).collect();
    for h in hs { h.join().unwrap(); }
    println!("spin total {}", shared.load(Ordering::SeqCst));

    let ready = Arc::new(AtomicBool::new(false));
    let data = Arc::new(AtomicI32::new(0));
    let (r2, d2) = (ready.clone(), data.clone());
    let p = thread::spawn(move || {
        d2.store(123, Ordering::Relaxed);
        r2.store(true, Ordering::Release);
    });
    while !ready.load(Ordering::Acquire) { std::hint::spin_loop(); }
    println!("published {}", data.load(Ordering::Relaxed));
    p.join().unwrap();
    let mut x = 5i32;
    let mut y = 6i32;
    let ap = AtomicPtr::new(&mut x as *mut i32);
    let old = ap.swap(&mut y, Ordering::SeqCst);
    unsafe { println!("atomicptr old {} new {}", *old, *ap.load(Ordering::SeqCst)); }
    std::sync::atomic::fence(Ordering::SeqCst);
    println!("bool fetch_not {} -> {}", ready.fetch_not(Ordering::SeqCst), ready.load(Ordering::SeqCst));
}
