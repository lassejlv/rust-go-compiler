use std::sync::atomic::{AtomicBool, AtomicI64, AtomicU32, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

static GLOBAL_COUNT: AtomicUsize = AtomicUsize::new(0);
static INIT_DONE: AtomicBool = AtomicBool::new(false);

fn next_id() -> usize {
    GLOBAL_COUNT.fetch_add(1, Ordering::SeqCst)
}

fn main() {
    let hs: Vec<_> = (0..8)
        .map(|_| {
            thread::spawn(|| {
                for _ in 0..500 {
                    next_id();
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("global count {}", GLOBAL_COUNT.load(Ordering::SeqCst));

    let first = INIT_DONE.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
    let second = INIT_DONE.compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst);
    println!("{:?} {:?}", first, second);

    let winners = Arc::new(AtomicU32::new(0));
    let flag = Arc::new(AtomicBool::new(false));
    let hs: Vec<_> = (0..6)
        .map(|_| {
            let w = Arc::clone(&winners);
            let f = Arc::clone(&flag);
            thread::spawn(move || {
                if !f.swap(true, Ordering::SeqCst) {
                    w.fetch_add(1, Ordering::SeqCst);
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("exactly one winner: {}", winners.load(Ordering::SeqCst) == 1);

    let acc = Arc::new(AtomicI64::new(0));
    let hs: Vec<_> = (1..=4)
        .map(|k| {
            let a = Arc::clone(&acc);
            thread::spawn(move || {
                for i in 0..100 {
                    if i % 2 == 0 {
                        a.fetch_add(k, Ordering::Relaxed);
                    } else {
                        a.fetch_sub(1, Ordering::Relaxed);
                    }
                }
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("acc {}", acc.load(Ordering::SeqCst));

    let maxv = Arc::new(AtomicUsize::new(0));
    let hs: Vec<_> = [17usize, 4, 99, 23, 56]
        .into_iter()
        .map(|v| {
            let m = Arc::clone(&maxv);
            thread::spawn(move || {
                m.fetch_max(v, Ordering::SeqCst);
            })
        })
        .collect();
    for h in hs {
        h.join().unwrap();
    }
    println!("max {}", maxv.load(Ordering::SeqCst));

    let bits = AtomicU32::new(0b1100);
    println!("{:#b}", bits.fetch_or(0b0011, Ordering::SeqCst));
    println!("{:#b}", bits.fetch_and(0b1010, Ordering::SeqCst));
    println!("{:#b}", bits.fetch_xor(0b1111, Ordering::SeqCst));
    println!("{:#b}", bits.load(Ordering::SeqCst));
    let r = bits.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| if x < 100 { Some(x * 10) } else { None });
    println!("{:?} -> {}", r, bits.load(Ordering::SeqCst));
    bits.store(7, Ordering::Release);
    println!("{}", bits.load(Ordering::Acquire));
    let mut local = AtomicUsize::new(3);
    *local.get_mut() += 4;
    println!("{} {}", local.load(Ordering::Relaxed), local.into_inner());
    println!("{:?}", AtomicBool::new(true));
}
