// Atomics: all orderings, fetch_update, compare_exchange_weak loops, AtomicPtr, AtomicBool ops.
use std::sync::atomic::{
    fence, AtomicBool, AtomicI32, AtomicI64, AtomicIsize, AtomicPtr, AtomicU16, AtomicU8, AtomicUsize, Ordering,
};
use std::sync::Arc;
use std::thread;

fn atomic_max(a: &AtomicI64, v: i64) {
    let mut cur = a.load(Ordering::Relaxed);
    loop {
        if v <= cur {
            return;
        }
        match a.compare_exchange_weak(cur, v, Ordering::AcqRel, Ordering::Relaxed) {
            Ok(_) => return,
            Err(actual) => cur = actual,
        }
    }
}

struct SpinLock {
    locked: AtomicBool,
}
impl SpinLock {
    fn lock(&self) {
        while self
            .locked
            .compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed)
            .is_err()
        {
            std::hint::spin_loop();
        }
    }
    fn unlock(&self) {
        self.locked.store(false, Ordering::Release);
    }
}

fn main() {
    let mx = Arc::new(AtomicI64::new(i64::MIN));
    let hs: Vec<_> = (0..6)
        .map(|t| {
            let mx = mx.clone();
            thread::spawn(move || {
                for i in 0..500 {
                    atomic_max(&mx, (i * 7 + t * 13) % 1000);
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("max {}", mx.load(Ordering::SeqCst));

    let lock = Arc::new(SpinLock { locked: AtomicBool::new(false) });
    let shared = Arc::new(AtomicUsize::new(0));
    let hs: Vec<_> = (0..4)
        .map(|_| {
            let lock = lock.clone();
            let shared = shared.clone();
            thread::spawn(move || {
                for _ in 0..1000 {
                    lock.lock();
                    let v = shared.load(Ordering::Relaxed);
                    shared.store(v + 1, Ordering::Relaxed);
                    lock.unlock();
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("spin total {}", shared.load(Ordering::SeqCst));

    let a = AtomicU8::new(250);
    println!("{:?}", a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| x.checked_add(3)));
    println!("{:?}", a.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| x.checked_add(3)));
    println!("wrapping add {} -> {}", a.fetch_add(10, Ordering::Relaxed), a.load(Ordering::Relaxed));
    let b = AtomicI32::new(5);
    println!("sub {} and {} or {} xor {} nand {}",
        b.fetch_sub(8, Ordering::AcqRel),
        b.fetch_and(0x7f, Ordering::Release),
        b.fetch_or(0x100, Ordering::Acquire),
        b.fetch_xor(0xff, Ordering::SeqCst),
        b.fetch_nand(0x0f, Ordering::Relaxed));
    println!("b now {}", b.load(Ordering::SeqCst));
    println!("min {} max {}", b.fetch_min(-1000, Ordering::SeqCst), b.fetch_max(42, Ordering::SeqCst));
    println!("b now {}", b.into_inner());
    let c = AtomicU16::new(7);
    println!("swap {} cx {:?} {:?}", c.swap(9, Ordering::SeqCst),
        c.compare_exchange(9, 11, Ordering::SeqCst, Ordering::SeqCst),
        c.compare_exchange(9, 12, Ordering::SeqCst, Ordering::SeqCst));
    let flag = AtomicBool::new(true);
    println!("bool ops {} {} {} {}", flag.fetch_and(false, Ordering::SeqCst), flag.fetch_or(true, Ordering::SeqCst),
        flag.fetch_xor(true, Ordering::SeqCst), flag.fetch_nand(true, Ordering::SeqCst));
    println!("flag {}", flag.load(Ordering::SeqCst));
    let mut is = AtomicIsize::new(-3);
    *is.get_mut() *= 4;
    println!("isize {}", is.load(Ordering::Relaxed));

    let mut x = 10i32;
    let mut y = 20i32;
    let p = AtomicPtr::new(&mut x as *mut i32);
    let old = p.swap(&mut y, Ordering::AcqRel);
    unsafe {
        *old += 1;
        *p.load(Ordering::Acquire) += 2;
    }
    fence(Ordering::SeqCst);
    println!("x={} y={}", x, y);

    // message passing with Release/Acquire
    let data = Arc::new(AtomicUsize::new(0));
    let ready = Arc::new(AtomicBool::new(false));
    let (d2, r2) = (data.clone(), ready.clone());
    let h = thread::spawn(move || {
        d2.store(1234, Ordering::Relaxed);
        r2.store(true, Ordering::Release);
    });
    while !ready.load(Ordering::Acquire) {
        std::hint::spin_loop();
    }
    println!("data {}", data.load(Ordering::Relaxed));
    h.join().unwrap();
    let arr: [AtomicUsize; 3] = Default::default();
    arr[1].store(5, Ordering::Relaxed);
    println!("{:?}", arr);
}
