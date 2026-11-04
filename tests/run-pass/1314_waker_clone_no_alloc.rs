// Creating, cloning and dropping Arc-based wakers and polling with Waker::noop must not
// allocate (checked with a counting #[global_allocator]); many polls of a future whose
// waker is re-registered every time.
use std::alloc::{GlobalAlloc, Layout, System};
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Counting;
static ALLOCS: AtomicUsize = AtomicUsize::new(0);
unsafe impl GlobalAlloc for Counting {
    unsafe fn alloc(&self, l: Layout) -> *mut u8 {
        ALLOCS.fetch_add(1, Ordering::SeqCst);
        System.alloc(l)
    }
    unsafe fn dealloc(&self, p: *mut u8, l: Layout) {
        System.dealloc(p, l)
    }
}
#[global_allocator]
static G: Counting = Counting;

struct Flag(AtomicUsize);
impl Wake for Flag {
    fn wake(self: Arc<Self>) {
        self.0.fetch_add(1, Ordering::Relaxed);
    }
}

struct Register {
    stored: Option<Waker>,
    left: u32,
}
impl Future for Register {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        if self.left == 0 {
            return Poll::Ready(7);
        }
        self.left -= 1;
        match &self.stored {
            Some(w) if w.will_wake(cx.waker()) => {}
            _ => self.stored = Some(cx.waker().clone()),
        }
        self.stored.as_ref().unwrap().wake_by_ref();
        Poll::Pending
    }
}

fn main() {
    let flag = Arc::new(Flag(AtomicUsize::new(0)));
    let waker = Waker::from(flag.clone());
    let before = ALLOCS.load(Ordering::SeqCst);
    let mut ws = [const { None }; 8];
    for i in 0..1000 {
        let w = waker.clone();
        if i % 125 == 0 {
            ws[i / 125] = Some(w.clone());
        }
        w.wake_by_ref();
        drop(w);
    }
    let w2 = Waker::from(flag.clone());
    let same = w2.will_wake(&waker);
    drop(w2);
    let mut cx = Context::from_waker(&waker);
    let mut fut = Register { stored: None, left: 500 };
    let mut polls = 0;
    let mut fp = Pin::new(&mut fut);
    let out = loop {
        polls += 1;
        if let Poll::Ready(v) = fp.as_mut().poll(&mut cx) {
            break v;
        }
    };
    let noop_cx = Context::from_waker(Waker::noop());
    let n2 = noop_cx.waker().clone();
    drop(n2);
    let after = ALLOCS.load(Ordering::SeqCst);
    drop(ws);
    println!("allocations while cloning/waking: {}", after - before);
    println!("same-type wakers will_wake: {}", same);
    println!("polls {} out {} wakes {}", polls, out, flag.0.load(Ordering::Relaxed));
    println!("strong count back to {}", Arc::strong_count(&flag) - 1);
}
