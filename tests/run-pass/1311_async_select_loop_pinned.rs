// select!-like polling inside async fns: pin!-ed locals polled through poll_fn across
// several loop iterations, `(&mut fut).await` on Unpin futures, Option<Pin<&mut F>>
// fusing, and Pin<&mut dyn Future> trait objects.
use std::future::{poll_fn, Future};
use std::pin::{pin, Pin};
use std::task::{Context, Poll, Waker};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

struct Ticks {
    name: &'static str,
    left: u32,
}
impl Future for Ticks {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<&'static str> {
        if self.left == 0 {
            Poll::Ready(self.name)
        } else {
            self.left -= 1;
            Poll::Pending
        }
    }
}

async fn slow(n: u32, tag: &'static str) -> String {
    for _ in 0..n {
        Ticks { name: "", left: 1 }.await;
    }
    format!("{}:{}", tag, n)
}

enum Which {
    A(String),
    B(String),
}

async fn race_all() -> Vec<String> {
    let mut a = pin!(slow(3, "a"));
    let mut b = pin!(slow(1, "b"));
    let mut a_done = false;
    let mut b_done = false;
    let mut order = Vec::new();
    while !(a_done && b_done) {
        let w = poll_fn(|cx| {
            if !a_done {
                if let Poll::Ready(v) = a.as_mut().poll(cx) {
                    return Poll::Ready(Which::A(v));
                }
            }
            if !b_done {
                if let Poll::Ready(v) = b.as_mut().poll(cx) {
                    return Poll::Ready(Which::B(v));
                }
            }
            Poll::Pending
        })
        .await;
        match w {
            Which::A(v) => {
                a_done = true;
                order.push(v)
            }
            Which::B(v) => {
                b_done = true;
                order.push(v)
            }
        }
    }
    order
}

async fn reuse_unpin() -> Vec<&'static str> {
    let mut t = Ticks { name: "reused", left: 2 };
    let mut out = vec![];
    // await a borrowed Unpin future, then use it again
    out.push((&mut t).await);
    t.left = 1;
    t.name = "again";
    out.push((&mut t).await);
    out
}

async fn drive_dyn(fs: Vec<Pin<Box<dyn Future<Output = String>>>>) -> Vec<String> {
    let mut out = vec![];
    for mut f in fs {
        let r: Pin<&mut dyn Future<Output = String>> = f.as_mut();
        out.push(r.await);
    }
    out
}

async fn fused() -> (u32, u32) {
    let mut slot: Option<Pin<Box<dyn Future<Output = &'static str>>>> = Some(Box::pin(Ticks { name: "x", left: 3 }));
    let mut polls = 0;
    let mut readies = 0;
    for _ in 0..6 {
        poll_fn(|cx| {
            polls += 1;
            if let Some(f) = slot.as_mut() {
                if f.as_mut().poll(cx).is_ready() {
                    readies += 1;
                    slot = None;
                }
            }
            Poll::Ready(())
        })
        .await;
    }
    (polls, readies)
}

fn main() {
    println!("{:?}", block_on(race_all()));
    println!("{:?}", block_on(reuse_unpin()));
    println!("{:?}", block_on(drive_dyn(vec![Box::pin(slow(2, "d1")), Box::pin(async { "d2".to_string() })])));
    println!("{:?}", block_on(fused()));
    let mut boxed: Box<dyn Future<Output = &'static str> + Unpin> = Box::new(Ticks { name: "boxed-unpin", left: 2 });
    println!("{}", block_on(&mut boxed));
}
