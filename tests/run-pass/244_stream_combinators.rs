// A futures::Stream-like trait with poll_next and an extension trait of combinators
// (map, filter, take, then, fold, collect, next) implemented as hand-written stream adapters.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

impl<S: Stream + Unpin + ?Sized> Stream for Box<S> {
    type Item = S::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        Pin::new(&mut **self).poll_next(cx)
    }
}

// Source: yields items from an iterator, returning Pending every other poll to simulate latency.
struct Iter<I> {
    it: I,
    toggle: bool,
}
impl<I: Iterator + Unpin> Stream for Iter<I> {
    type Item = I::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<I::Item>> {
        self.toggle = !self.toggle;
        if self.toggle {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.it.next())
    }
}
fn iter<I: IntoIterator>(i: I) -> Iter<I::IntoIter> {
    Iter { it: i.into_iter(), toggle: false }
}

struct Map<S, F> {
    s: S,
    f: F,
}
impl<S: Stream + Unpin, F: FnMut(S::Item) -> T + Unpin, T> Stream for Map<S, F> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let this = &mut *self;
        match Pin::new(&mut this.s).poll_next(cx) {
            Poll::Ready(Some(x)) => Poll::Ready(Some((this.f)(x))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct Filter<S, P> {
    s: S,
    p: P,
}
impl<S: Stream + Unpin, P: FnMut(&S::Item) -> bool + Unpin> Stream for Filter<S, P> {
    type Item = S::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        let this = &mut *self;
        loop {
            match Pin::new(&mut this.s).poll_next(cx) {
                Poll::Ready(Some(x)) => {
                    if (this.p)(&x) {
                        return Poll::Ready(Some(x));
                    }
                }
                other => return other,
            }
        }
    }
}

struct Take<S> {
    s: S,
    left: usize,
}
impl<S: Stream + Unpin> Stream for Take<S> {
    type Item = S::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<S::Item>> {
        if self.left == 0 {
            return Poll::Ready(None);
        }
        let r = Pin::new(&mut self.s).poll_next(cx);
        if let Poll::Ready(Some(_)) = &r {
            self.left -= 1;
        }
        r
    }
}

// `then`: map each item through an async function
struct Then<S, F, Fut> {
    s: S,
    f: F,
    pending: Option<Pin<Box<Fut>>>,
}
impl<S, F, Fut> Stream for Then<S, F, Fut>
where
    S: Stream + Unpin,
    F: FnMut(S::Item) -> Fut + Unpin,
    Fut: Future,
{
    type Item = Fut::Output;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Fut::Output>> {
        let this = &mut *self;
        loop {
            if let Some(fut) = this.pending.as_mut() {
                let out = match fut.as_mut().poll(cx) {
                    Poll::Ready(v) => v,
                    Poll::Pending => return Poll::Pending,
                };
                this.pending = None;
                return Poll::Ready(Some(out));
            }
            match Pin::new(&mut this.s).poll_next(cx) {
                Poll::Ready(Some(x)) => this.pending = Some(Box::pin((this.f)(x))),
                Poll::Ready(None) => return Poll::Ready(None),
                Poll::Pending => return Poll::Pending,
            }
        }
    }
}

struct Next<'a, S: ?Sized>(&'a mut S);
impl<S: Stream + Unpin + ?Sized> Future for Next<'_, S> {
    type Output = Option<S::Item>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.0).poll_next(cx)
    }
}

trait StreamExt: Stream {
    fn map<T, F: FnMut(Self::Item) -> T>(self, f: F) -> Map<Self, F>
    where
        Self: Sized,
    {
        Map { s: self, f }
    }
    fn filter<P: FnMut(&Self::Item) -> bool>(self, p: P) -> Filter<Self, P>
    where
        Self: Sized,
    {
        Filter { s: self, p }
    }
    fn take(self, n: usize) -> Take<Self>
    where
        Self: Sized,
    {
        Take { s: self, left: n }
    }
    fn then<F, Fut>(self, f: F) -> Then<Self, F, Fut>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> Fut,
        Fut: Future,
    {
        Then { s: self, f, pending: None }
    }
    fn next(&mut self) -> Next<'_, Self>
    where
        Self: Unpin,
    {
        Next(self)
    }
    fn boxed<'a>(self) -> Box<dyn Stream<Item = Self::Item> + Unpin + 'a>
    where
        Self: Sized + Unpin + 'a,
    {
        Box::new(self)
    }
}
impl<S: Stream + ?Sized> StreamExt for S {}

async fn collect<S: Stream + Unpin>(mut s: S) -> Vec<S::Item> {
    let mut v = vec![];
    while let Some(x) = s.next().await {
        v.push(x);
    }
    v
}

async fn fold<S: Stream + Unpin, A>(mut s: S, init: A, mut f: impl FnMut(A, S::Item) -> A) -> A {
    let mut acc = init;
    while let Some(x) = s.next().await {
        acc = f(acc, x);
    }
    acc
}

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}
fn block_on<F: Future>(f: F) -> (F::Output, usize) {
    let w = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&w);
    let mut f = Box::pin(f);
    let mut n = 0;
    loop {
        n += 1;
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return (v, n);
        }
    }
}

async fn slow_square(x: u64) -> u64 {
    let mut yielded = false;
    std::future::poll_fn(|cx| {
        if yielded {
            Poll::Ready(())
        } else {
            yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await;
    x * x
}

fn main() {
    let (v, polls) = block_on(collect(iter(1..=10).filter(|x| x % 3 != 0).map(|x| x * 10).take(5)));
    println!("{:?} polls={}", v, polls);

    let (v, polls) = block_on(collect(iter(vec!["a", "bb", "ccc"]).then(|s| async move { s.len() * 100 })));
    println!("{:?} polls={}", v, polls);

    let (sum, polls) = block_on(fold(iter(1..=6u64).then(slow_square), 0, |a, x| a + x));
    println!("sum of squares {} polls={}", sum, polls);

    let streams: Vec<Box<dyn Stream<Item = String> + Unpin>> = vec![
        iter(0..3).map(|i| format!("n{}", i)).boxed(),
        iter("hello world".split(' ')).map(str::to_uppercase).boxed(),
        iter(Vec::<String>::new()).boxed(),
    ];
    for (i, s) in streams.into_iter().enumerate() {
        let (v, polls) = block_on(collect(s));
        println!("stream {} -> {:?} polls={}", i, v, polls);
    }

    let (r, _) = block_on(async {
        let mut s = iter(10..).map(|x| x * 2);
        let a = s.next().await;
        let b = s.next().await;
        let rest = collect(s.take(3)).await;
        (a, b, rest)
    });
    println!("{:?}", r);
}
