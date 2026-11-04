// RawWaker with a custom vtable that counts clones/wakes/drops via statics.
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

static CLONES: AtomicUsize = AtomicUsize::new(0);
static WAKES: AtomicUsize = AtomicUsize::new(0);
static WAKE_BY_REF: AtomicUsize = AtomicUsize::new(0);
static DROPS: AtomicUsize = AtomicUsize::new(0);

unsafe fn vt_clone(p: *const ()) -> RawWaker {
    CLONES.fetch_add(1, Ordering::Relaxed);
    RawWaker::new(p, &VTABLE)
}
unsafe fn vt_wake(p: *const ()) {
    WAKES.fetch_add(1, Ordering::Relaxed);
    vt_drop(p);
}
unsafe fn vt_wake_by_ref(p: *const ()) {
    let flag = &*(p as *const AtomicUsize);
    flag.fetch_add(1, Ordering::Relaxed);
    WAKE_BY_REF.fetch_add(1, Ordering::Relaxed);
}
unsafe fn vt_drop(_p: *const ()) {
    DROPS.fetch_add(1, Ordering::Relaxed);
}

static VTABLE: RawWakerVTable = RawWakerVTable::new(vt_clone, vt_wake, vt_wake_by_ref, vt_drop);

fn block_on<F: Future>(fut: F) -> F::Output {
    let flag = AtomicUsize::new(0);
    let raw = RawWaker::new(&flag as *const AtomicUsize as *const (), &VTABLE);
    let waker = unsafe { Waker::from_raw(raw) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    let mut n = 0;
    loop {
        n += 1;
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => {
                println!("block_on: {} polls, flag={}", n, flag.load(Ordering::Relaxed));
                return v;
            }
            Poll::Pending => {}
        }
    }
}

struct StoreWaker {
    stored: Option<Waker>,
    step: u8,
}

impl Future for StoreWaker {
    type Output = u8;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u8> {
        self.step += 1;
        match self.step {
            1 => {
                self.stored = Some(cx.waker().clone());
                Poll::Pending
            }
            2 => {
                // wake the stored clone (consumes it)
                if let Some(w) = self.stored.take() {
                    w.wake();
                }
                cx.waker().wake_by_ref();
                Poll::Pending
            }
            3 => {
                let w = cx.waker().clone();
                println!("will_wake same: {}", w.will_wake(cx.waker()));
                drop(w);
                Poll::Ready(self.step * 10)
            }
            _ => unreachable!(),
        }
    }
}

async fn chain() -> u32 {
    let a = StoreWaker { stored: None, step: 0 }.await;
    let b = StoreWaker { stored: None, step: 0 }.await;
    a as u32 + b as u32
}

fn main() {
    let v = block_on(chain());
    println!("v={}", v);
    println!(
        "clones={} wakes={} wake_by_ref={} drops={}",
        CLONES.load(Ordering::Relaxed),
        WAKES.load(Ordering::Relaxed),
        WAKE_BY_REF.load(Ordering::Relaxed),
        DROPS.load(Ordering::Relaxed)
    );
    let noop = Waker::noop();
    let mut cx = Context::from_waker(noop);
    let mut f = Box::pin(async { 5 + 6 });
    println!("{:?}", f.as_mut().poll(&mut cx));
}
