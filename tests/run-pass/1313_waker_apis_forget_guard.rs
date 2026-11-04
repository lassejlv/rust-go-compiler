// Waker::data/vtable/will_wake, RawWaker from Arc<impl Wake>, Waker::noop cloning;
// forgetting a MutexGuard keeps the mutex locked; ManuallyDrop guards; scoped-thread
// panic payload; Barrier reused across generations.
use std::mem::{self, ManuallyDrop};
use std::panic;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Barrier, Mutex};
use std::task::{RawWaker, Wake, Waker};
use std::thread;

struct W(AtomicUsize);
impl Wake for W {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}

fn main() {
    let w = Arc::new(W(AtomicUsize::new(0)));
    let waker = Waker::from(w.clone());
    let w2 = waker.clone();
    println!("will_wake clone {}", waker.will_wake(&w2));
    let other = Waker::from(Arc::new(W(AtomicUsize::new(0))));
    println!("will_wake other {}", waker.will_wake(&other));
    println!("data is arc ptr {}", waker.data() == Arc::as_ptr(&w) as *const ());
    println!("same vtable {}", std::ptr::eq(waker.vtable(), w2.vtable()));
    println!("strong count with 2 wakers {}", Arc::strong_count(&w));
    w2.wake_by_ref();
    w2.wake();
    waker.wake();
    println!("wakes {} strong now {}", w.0.load(Ordering::SeqCst), Arc::strong_count(&w));
    let raw: RawWaker = w.clone().into();
    let from_raw = unsafe { Waker::from_raw(raw) };
    from_raw.wake();
    println!("wakes via RawWaker {} strong {}", w.0.load(Ordering::SeqCst), Arc::strong_count(&w));
    let n1 = Waker::noop().clone();
    println!("noop will_wake noop {}", n1.will_wake(Waker::noop()));
    drop(other);

    let m = Mutex::new(5);
    let g = m.lock().unwrap();
    mem::forget(g);
    println!("after forget try_lock is_err {}", m.try_lock().is_err());
    let m2 = Mutex::new(vec![1]);
    let mut md = ManuallyDrop::new(m2.lock().unwrap());
    md.push(2);
    println!("manually drop guard holds lock: {}", m2.try_lock().is_err());
    unsafe { ManuallyDrop::drop(&mut md) };
    println!("after drop {:?}", m2.lock().unwrap());

    panic::set_hook(Box::new(|_| {}));
    let r = panic::catch_unwind(|| {
        thread::scope(|s| {
            s.spawn(|| panic!("inner scoped panic"));
            s.spawn(|| 5);
        })
    });
    let e = r.unwrap_err();
    println!("scope payload: {:?}", e.downcast_ref::<&str>());
    let _ = panic::take_hook();

    let n = 3;
    let barrier = Arc::new(Barrier::new(n));
    let counter = Arc::new(AtomicUsize::new(0));
    let hs: Vec<_> = (0..n)
        .map(|_| {
            let b = barrier.clone();
            let c = counter.clone();
            thread::spawn(move || {
                let mut seen = vec![];
                let mut leaders = 0;
                for round in 0..4 {
                    c.fetch_add(1, Ordering::SeqCst);
                    if b.wait().is_leader() {
                        leaders += 1;
                    }
                    seen.push(c.load(Ordering::SeqCst) >= (round + 1) * n);
                    b.wait();
                }
                (seen, leaders)
            })
        })
        .collect();
    let mut total_leaders = 0;
    for h in hs {
        let (seen, l) = h.join().unwrap();
        total_leaders += l;
        println!("{:?}", seen);
    }
    println!("leaders total {} counter {}", total_leaders, counter.load(Ordering::SeqCst));
}
