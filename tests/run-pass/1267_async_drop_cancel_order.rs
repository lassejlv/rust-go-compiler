// Drop order of locals inside futures cancelled at various suspension points.
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Noisy(&'static str);
impl Drop for Noisy {
    fn drop(&mut self) {
        println!("drop {}", self.0);
    }
}

struct Pending1(bool);
impl Future for Pending1 {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; Poll::Pending }
    }
}

async fn inner(tag: &'static str) -> u32 {
    let _i = Noisy(tag);
    Pending1(false).await;
    println!("inner {} resumed", tag);
    7
}

async fn outer() -> u32 {
    let _a = Noisy("a");
    let x = inner("i1").await;
    let _b = Noisy("b");
    {
        let _scoped = Noisy("scoped");
        Pending1(false).await;
    }
    let y = inner("i2").await;
    let _c = Noisy("c");
    x + y
}

fn poll_n<F: Future>(f: F, n: usize) -> Option<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = Box::pin(f);
    for i in 0..n {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            println!("ready after {} polls", i + 1);
            return Some(v);
        }
    }
    println!("cancelling after {} polls", n);
    None
}

fn main() {
    for n in 0..6 {
        println!("== n={}", n);
        let r = poll_n(outer(), n);
        println!("result {:?}", r);
    }
    println!("== never polled with captured value");
    let captured = Noisy("captured");
    let f = async move {
        let _keep = &captured;
        Pending1(false).await;
    };
    drop(f);
    println!("== async block captured by ref");
    let local = Noisy("local");
    {
        let f = async {
            let _r = &local;
            let _t = Noisy("temp-in-block");
            Pending1(false).await;
        };
        poll_n(f, 1);
    }
    println!("end main");
}
