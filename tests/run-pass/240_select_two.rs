// select-like polling of two futures, Either outputs, timeouts against a virtual clock, and
// FusedFuture-style re-polling in a loop.
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

#[derive(Debug)]
enum Either<A, B> {
    Left(A),
    Right(B),
}

struct Select<A, B> {
    a: Option<Pin<Box<A>>>,
    b: Option<Pin<Box<B>>>,
}

fn select<A: Future, B: Future>(a: A, b: B) -> Select<A, B> {
    Select { a: Some(Box::pin(a)), b: Some(Box::pin(b)) }
}

impl<A: Future, B: Future> Future for Select<A, B> {
    // the winner's output plus the still-pending loser
    type Output = Either<(A::Output, Pin<Box<B>>), (B::Output, Pin<Box<A>>)>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = self.a.as_mut().unwrap().as_mut().poll(cx) {
            let b = self.b.take().unwrap();
            return Poll::Ready(Either::Left((v, b)));
        }
        if let Poll::Ready(v) = self.b.as_mut().unwrap().as_mut().poll(cx) {
            let a = self.a.take().unwrap();
            return Poll::Ready(Either::Right((v, a)));
        }
        Poll::Pending
    }
}

// Virtual clock: each executor tick advances time by 1.
#[derive(Clone)]
struct Clock(Rc<Cell<u64>>);

struct Sleep {
    clock: Clock,
    until: u64,
}
impl Future for Sleep {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        let now = self.clock.0.get();
        if now >= self.until {
            Poll::Ready(now)
        } else {
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

impl Clock {
    fn sleep(&self, d: u64) -> Sleep {
        Sleep { clock: self.clone(), until: self.0.get() + d }
    }
    fn now(&self) -> u64 {
        self.0.get()
    }
}

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn run<F: Future>(clock: &Clock, f: F) -> F::Output {
    let w = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&w);
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
        clock.0.set(clock.0.get() + 1);
    }
}

async fn work(clock: Clock, name: &'static str, cost: u64) -> String {
    let start = clock.now();
    clock.sleep(cost).await;
    format!("{} finished at {} (started {})", name, clock.now(), start)
}

async fn timeout<F: Future>(clock: &Clock, limit: u64, f: F) -> Result<F::Output, u64> {
    match select(f, clock.sleep(limit)).await {
        Either::Left((v, _)) => Ok(v),
        Either::Right((t, _)) => Err(t),
    }
}

fn main() {
    let clock = Clock(Rc::new(Cell::new(0)));

    let c = clock.clone();
    let out = run(&clock, async move {
        match select(work(c.clone(), "fast", 3), work(c.clone(), "slow", 8)).await {
            Either::Left((w, rest)) => {
                println!("left won: {}", w);
                let r = rest.await;
                println!("then: {}", r);
            }
            Either::Right((w, _)) => println!("right won: {}", w),
        }
        c.now()
    });
    println!("t={}", out);

    let c = clock.clone();
    run(&clock, async move {
        for (cost, limit) in [(2, 5), (10, 4), (4, 4)] {
            let r = timeout(&c, limit, work(c.clone(), "job", cost)).await;
            match r {
                Ok(s) => println!("ok: {}", s),
                Err(t) => println!("timed out at {} (cost {}, limit {})", t, cost, limit),
            }
        }
    });

    // select in a loop: a ticker vs a long job, re-using the pending loser each iteration
    let c = clock.clone();
    let ticks = run(&clock, async move {
        let mut job: Pin<Box<_>> = Box::pin(work(c.clone(), "long", 7));
        let mut ticks = 0;
        loop {
            match select(job, c.sleep(2)).await {
                Either::Left((s, _)) => {
                    println!("job: {}", s);
                    break;
                }
                Either::Right((t, rest)) => {
                    ticks += 1;
                    println!("tick {} at {}", ticks, t);
                    job = *Pin::into_inner(rest);
                }
            }
        }
        ticks
    });
    println!("ticks={} final t={}", ticks, clock.now());

    let r = run(&clock, select(std::future::ready(1), std::future::ready("b")));
    match r {
        Either::Left((v, _)) => println!("ready left {}", v),
        Either::Right((v, _)) => println!("ready right {}", v),
    }
    let r = run(&clock, select(std::future::pending::<()>(), std::future::ready('z')));
    if let Either::Right((v, _)) = r {
        println!("pending loses to {:?}", v);
    }
}
