// parking_lot-style primitives on atomics: a spin lock with RAII guard (UnsafeCell + AtomicBool),
// a seqlock-ish counter, a ticket lock, and an atomic bitmask; final values are deterministic.
use std::cell::UnsafeCell;
use std::hint;
use std::ops::{Deref, DerefMut};
use std::sync::atomic::{AtomicBool, AtomicU32, AtomicU64, AtomicUsize, Ordering};
use std::sync::Arc;
use std::thread;

struct SpinLock<T> {
    locked: AtomicBool,
    contended: AtomicUsize,
    data: UnsafeCell<T>,
}

unsafe impl<T: Send> Sync for SpinLock<T> {}
unsafe impl<T: Send> Send for SpinLock<T> {}

struct SpinGuard<'a, T> {
    lock: &'a SpinLock<T>,
}

impl<T> SpinLock<T> {
    const fn new(v: T) -> Self {
        SpinLock { locked: AtomicBool::new(false), contended: AtomicUsize::new(0), data: UnsafeCell::new(v) }
    }
    fn lock(&self) -> SpinGuard<'_, T> {
        let mut spun = false;
        while self.locked.compare_exchange_weak(false, true, Ordering::Acquire, Ordering::Relaxed).is_err() {
            spun = true;
            while self.locked.load(Ordering::Relaxed) {
                hint::spin_loop();
            }
        }
        if spun {
            self.contended.fetch_add(1, Ordering::Relaxed);
        }
        SpinGuard { lock: self }
    }
    fn try_lock(&self) -> Option<SpinGuard<'_, T>> {
        self.locked.compare_exchange(false, true, Ordering::Acquire, Ordering::Relaxed).ok().map(|_| SpinGuard { lock: self })
    }
    fn into_inner(self) -> T {
        self.data.into_inner()
    }
}

impl<T> Deref for SpinGuard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.lock.data.get() }
    }
}
impl<T> DerefMut for SpinGuard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.lock.data.get() }
    }
}
impl<T> Drop for SpinGuard<'_, T> {
    fn drop(&mut self) {
        self.lock.locked.store(false, Ordering::Release);
    }
}

struct TicketLock {
    next: AtomicU32,
    serving: AtomicU32,
}

impl TicketLock {
    fn with<R>(&self, f: impl FnOnce() -> R) -> R {
        let my = self.next.fetch_add(1, Ordering::Relaxed);
        while self.serving.load(Ordering::Acquire) != my {
            hint::spin_loop();
        }
        let r = f();
        self.serving.fetch_add(1, Ordering::Release);
        r
    }
}

static GLOBAL: SpinLock<Vec<u32>> = SpinLock::new(Vec::new());

fn main() {
    let lock = Arc::new(SpinLock::new((0u64, Vec::<usize>::new())));
    let handles: Vec<_> = (0..4)
        .map(|t| {
            let lock = Arc::clone(&lock);
            thread::spawn(move || {
                for i in 0..1000 {
                    let mut g = lock.lock();
                    g.0 += (t * 1000 + i) as u64;
                    if i % 250 == 0 {
                        g.1.push(t);
                    }
                }
            })
        })
        .collect();
    for h in handles {
        h.join().unwrap();
    }
    let lock = Arc::try_unwrap(lock).ok().unwrap();
    let (sum, mut marks) = lock.into_inner();
    marks.sort();
    println!("sum={} expected={} marks={:?}", sum, (0..4000u64).sum::<u64>(), marks);

    let l2 = SpinLock::new(5);
    {
        let g = l2.lock();
        println!("try_lock while held: {}", l2.try_lock().is_some());
        println!("value {}", *g);
    }
    println!("try_lock after release: {}", l2.try_lock().map(|g| *g * 2).unwrap_or(-1));

    let ticket = Arc::new(TicketLock { next: AtomicU32::new(0), serving: AtomicU32::new(0) });
    let shared = Arc::new(AtomicU64::new(0));
    thread::scope(|s| {
        for t in 0..3u64 {
            let ticket = &ticket;
            let shared = &shared;
            s.spawn(move || {
                for _ in 0..500 {
                    ticket.with(|| {
                        // non-atomic read-modify-write made safe by the ticket lock
                        let v = shared.load(Ordering::Relaxed);
                        shared.store(v + t + 1, Ordering::Relaxed);
                    });
                }
            });
        }
    });
    println!("ticket total={} tickets issued={}", shared.load(Ordering::SeqCst), ticket.next.load(Ordering::SeqCst));

    let mask = Arc::new(AtomicU64::new(0));
    thread::scope(|s| {
        for t in 0..8u64 {
            let mask = &mask;
            s.spawn(move || {
                for bit in (t..64).step_by(8) {
                    mask.fetch_or(1 << bit, Ordering::AcqRel);
                }
                if t % 2 == 1 {
                    mask.fetch_and(!(1u64 << t), Ordering::AcqRel);
                }
            });
        }
    });
    let m = mask.load(Ordering::SeqCst);
    println!("mask={:016x} ones={}", m, m.count_ones());

    let counter = AtomicUsize::new(10);
    println!("{:?}", counter.compare_exchange(10, 20, Ordering::SeqCst, Ordering::SeqCst));
    println!("{:?}", counter.compare_exchange(10, 30, Ordering::SeqCst, Ordering::SeqCst));
    println!("{} {}", counter.fetch_sub(5, Ordering::SeqCst), counter.swap(1, Ordering::SeqCst));
    println!("{:?}", counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| if x < 5 { Some(x * 10) } else { None }));
    println!("{:?} {}", counter.fetch_update(Ordering::SeqCst, Ordering::SeqCst, |x| if x < 5 { Some(x * 10) } else { None }), counter.load(Ordering::SeqCst));
    println!("{} {}", counter.fetch_max(99, Ordering::SeqCst), counter.fetch_min(3, Ordering::SeqCst));
    println!("{}", counter.into_inner());

    thread::scope(|s| {
        for t in 0..4u32 {
            s.spawn(move || {
                for i in 0..5 {
                    GLOBAL.lock().push(t * 10 + i);
                }
            });
        }
    });
    let mut g = GLOBAL.lock();
    g.sort();
    println!("global {} items, first {:?} last {:?}", g.len(), g.first(), g.last());
}
