use std::future::Future;
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

struct D(&'static str);
impl Drop for D {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct YieldOnce(bool);
impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let _g = D("block_on frame");
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn inner(n: u32) -> u32 {
    let _d = D("inner local");
    YieldOnce(false).await;
    if n > 2 {
        panic!("inner panicked with {}", n);
    }
    n * 2
}

async fn outer(n: u32) -> u32 {
    let _d = D("outer local");
    let x = inner(n).await;
    x + 1
}

fn main() {
    println!("ok: {}", block_on(outer(1)));
    let r = catch_unwind(AssertUnwindSafe(|| block_on(outer(5))));
    match r {
        Ok(v) => println!("value {}", v),
        Err(e) => println!("caught: {}", e.downcast_ref::<String>().unwrap()),
    }
    // poll inside catch_unwind directly
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(async {
        let _d = D("block local");
        YieldOnce(false).await;
        panic!("static msg");
    });
    for i in 0..3 {
        let r = catch_unwind(AssertUnwindSafe(|| fut.as_mut().poll(&mut cx)));
        match r {
            Ok(Poll::Pending) => println!("poll {}: pending", i),
            Ok(Poll::Ready(())) => println!("poll {}: ready", i),
            Err(e) => println!("poll {}: caught {:?}", i, e.downcast_ref::<&str>()),
        }
    }
    // nested async blocks
    let r = catch_unwind(|| {
        block_on(async {
            let _a = D("outer block");
            async {
                let _b = D("inner block");
                YieldOnce(false).await;
                let v: Vec<u32> = Vec::new();
                v[3]
            }
            .await
        })
    });
    println!("nested: {}", r.is_err());
    // async fn polled again after panicking
    let mut f2 = Box::pin(inner(9));
    for i in 0..3 {
        let r = catch_unwind(AssertUnwindSafe(|| f2.as_mut().poll(&mut cx)));
        println!("f2 poll {}: {}", i, match r { Ok(p) => format!("{:?}", p), Err(e) => format!("err {:?} {:?}", e.downcast_ref::<String>(), e.downcast_ref::<&str>()) });
    }
    println!("end");
}
