// Hand-written Future combinators: Map, Join, Select, and a timer-like countdown.
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Countdown {
    name: &'static str,
    left: u32,
}
impl Future for Countdown {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if self.left == 0 {
            println!("  {} ready", self.name);
            Poll::Ready(self.name)
        } else {
            self.left -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
impl Drop for Countdown {
    fn drop(&mut self) {
        println!("  drop {} (left {})", self.name, self.left);
    }
}

struct Map<F, G> {
    fut: F,
    f: Option<G>,
}
impl<F: Future + Unpin, G: FnOnce(F::Output) -> T + Unpin, T> Future for Map<F, G> {
    type Output = T;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        match Pin::new(&mut self.fut).poll(cx) {
            Poll::Ready(v) => {
                let f = self.f.take().expect("polled after completion");
                Poll::Ready(f(v))
            }
            Poll::Pending => Poll::Pending,
        }
    }
}

enum MaybeDone<F: Future> {
    Pending(F),
    Done(F::Output),
    Taken,
}

struct Join<A: Future, B: Future> {
    a: MaybeDone<A>,
    b: MaybeDone<B>,
}

impl<A: Future + Unpin, B: Future + Unpin> Future for Join<A, B>
where
    A::Output: Unpin,
    B::Output: Unpin,
{
    type Output = (A::Output, B::Output);
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if let MaybeDone::Pending(f) = &mut this.a {
            if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                this.a = MaybeDone::Done(v);
            }
        }
        if let MaybeDone::Pending(f) = &mut this.b {
            if let Poll::Ready(v) = Pin::new(f).poll(cx) {
                this.b = MaybeDone::Done(v);
            }
        }
        if matches!(this.a, MaybeDone::Done(_)) && matches!(this.b, MaybeDone::Done(_)) {
            let a = match std::mem::replace(&mut this.a, MaybeDone::Taken) {
                MaybeDone::Done(v) => v,
                _ => unreachable!(),
            };
            let b = match std::mem::replace(&mut this.b, MaybeDone::Taken) {
                MaybeDone::Done(v) => v,
                _ => unreachable!(),
            };
            Poll::Ready((a, b))
        } else {
            Poll::Pending
        }
    }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

struct Select<A, B> {
    a: Option<A>,
    b: Option<B>,
}
impl<A: Future + Unpin, B: Future + Unpin> Future for Select<A, B> {
    type Output = Either<(A::Output, B), (B::Output, A)>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if let Poll::Ready(v) = Pin::new(this.a.as_mut().unwrap()).poll(cx) {
            this.a = None;
            return Poll::Ready(Either::Left((v, this.b.take().unwrap())));
        }
        if let Poll::Ready(v) = Pin::new(this.b.as_mut().unwrap()).poll(cx) {
            this.b = None;
            return Poll::Ready(Either::Right((v, this.a.take().unwrap())));
        }
        Poll::Pending
    }
}

fn block_on<F: Future>(f: F) -> (F::Output, u32) {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return (v, polls);
        }
    }
}

fn main() {
    println!("map:");
    let m = Map { fut: Countdown { name: "m", left: 2 }, f: Some(|s: &str| s.len() * 100) };
    println!("{:?}", block_on(m));

    println!("join:");
    let j = Join {
        a: MaybeDone::Pending(Countdown { name: "ja", left: 3 }),
        b: MaybeDone::Pending(Countdown { name: "jb", left: 1 }),
    };
    let ((a, b), polls) = block_on(j);
    println!("joined {} {} in {}", a, b, polls);

    println!("select:");
    let s = Select { a: Some(Countdown { name: "slow", left: 5 }), b: Some(Countdown { name: "fast", left: 2 }) };
    let (res, polls) = block_on(s);
    match res {
        Either::Left((v, rest)) => println!("left won {} rest {}", v, rest.name),
        Either::Right((v, rest)) => {
            println!("right won {} after {} polls; continuing loser", v, polls);
            let (v2, p2) = block_on(rest);
            println!("loser finished {} in {}", v2, p2);
        }
    }
    println!("select cancel:");
    let s = Select { a: Some(Countdown { name: "c1", left: 1 }), b: Some(Countdown { name: "c2", left: 4 }) };
    let (res, _) = block_on(s);
    if let Either::Left((v, rest)) = res {
        println!("won {}", v);
        drop(rest);
        println!("after drop");
    }
    println!("end");
}
