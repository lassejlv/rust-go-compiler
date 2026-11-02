use std::future::{self, Future};
use std::pin::{pin, Pin};
use std::task::{Context, Poll, Waker};

fn poll_once<F: Future + ?Sized>(f: Pin<&mut F>) -> Poll<F::Output> {
    let mut cx = Context::from_waker(Waker::noop());
    f.poll(&mut cx)
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = poll_once(f.as_mut()) {
            return v;
        }
    }
}

struct Counter {
    polls: u32,
    target: u32,
}

impl Future for Counter {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        let this = self.get_mut();
        this.polls += 1;
        if this.polls >= this.target {
            Poll::Ready(this.polls)
        } else {
            Poll::Pending
        }
    }
}

async fn add_ready(a: i32) -> i32 {
    let b = future::ready(10).await;
    let c = future::ready(a * 2).await;
    a + b + c
}

fn main() {
    let r = future::ready(5);
    let mut r = pin!(r);
    println!("{:?}", poll_once(r.as_mut()));

    let p = future::pending::<i32>();
    let mut p = pin!(p);
    println!("{:?}", poll_once(p.as_mut()));
    println!("{:?}", poll_once(p.as_mut()));

    let mut c = Counter { polls: 0, target: 3 };
    let mut pc = Pin::new(&mut c);
    println!("{:?}", poll_once(pc.as_mut()));
    println!("{:?}", poll_once(pc.as_mut()));
    println!("{:?}", poll_once(pc.as_mut()));
    println!("counter polls {}", c.polls);

    let fut = add_ready(7);
    let mut fut = pin!(fut);
    println!("{:?}", poll_once(fut.as_mut()));

    let mut boxed: Pin<Box<dyn Future<Output = String>>> = Box::pin(async { format!("{}-{}", "boxed", add_ready(1).await) });
    println!("{:?}", poll_once(boxed.as_mut()));

    let futs: Vec<Pin<Box<dyn Future<Output = i32>>>> = vec![
        Box::pin(future::ready(1)),
        Box::pin(add_ready(2)),
        Box::pin(async { Counter { polls: 0, target: 2 }.await as i32 * 100 }),
    ];
    let results: Vec<i32> = futs.into_iter().map(block_on).collect();
    println!("{:?}", results);

    let pf = future::poll_fn(|_cx| Poll::Ready("from poll_fn"));
    println!("{}", block_on(pf));
    let mut n = 0;
    let pf2 = future::poll_fn(move |_cx| {
        n += 1;
        if n < 4 { Poll::Pending } else { Poll::Ready(n) }
    });
    println!("poll_fn counted to {}", block_on(pf2));

    let x = 5;
    let mut v = vec![1, 2];
    let fut = async {
        v.push(x);
        v.len()
    };
    println!("len {}", block_on(fut));
    println!("{:?}", v);
    let s: Pin<Box<i32>> = Box::pin(42);
    let first = *s;
    println!("{} {}", first, Pin::into_inner(s));
    let mut val = 3;
    let mut pinned = Pin::new(&mut val);
    pinned.set(9);
    println!("{}", val);
}
