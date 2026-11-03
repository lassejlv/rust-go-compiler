use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

struct D(String);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct YieldOnce(bool);
impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            Poll::Pending
        }
    }
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn boom(tag: &str) -> u32 {
    let _d = D(format!("boom {}", tag));
    YieldOnce(false).await;
    panic!("boom {}", tag)
}

fn main() {
    // catch_unwind inside an async block around a nested future's poll
    let v = block_on(async {
        let _o = D("outer".to_string());
        let mut f = Box::pin(boom("a"));
        let waker = Waker::from(Arc::new(Noop));
        let mut cx = Context::from_waker(&waker);
        let mut n = 0;
        loop {
            let r = catch_unwind(AssertUnwindSafe(|| f.as_mut().poll(&mut cx)));
            match r {
                Ok(Poll::Pending) => {
                    YieldOnce(false).await;
                    n += 1;
                }
                Ok(Poll::Ready(v)) => break v,
                Err(_) => break 100 + n,
            }
        }
    });
    println!("v = {}", v);
    // panic payload from panic_any
    let r = catch_unwind(|| {
        block_on(async {
            YieldOnce(false).await;
            std::panic::panic_any(42i32);
        })
    });
    println!("any: {:?}", r.err().and_then(|e| e.downcast_ref::<i32>().copied()));
    // threads
    let h = std::thread::spawn(|| {
        let r = catch_unwind(|| block_on(boom("thread")));
        println!("thread caught: {}", r.is_err());
        block_on(boom("thread2"))
    });
    println!("join err: {}", h.join().is_err());
    // uncaught: runs main's destructors too
    let _m = D("main local".to_string());
    block_on(async {
        let _x = D("last".to_string());
        boom("final").await
    });
    println!("unreachable");
}
