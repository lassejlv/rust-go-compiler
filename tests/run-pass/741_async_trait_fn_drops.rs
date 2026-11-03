// async fn in traits (static dispatch), boxed recursive async, references held
// across await points, drop order of locals inside async fns (at completion
// and on cancellation by dropping the future), and a tiny select/join.
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

thread_local! { static LOG: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn log(s: impl Into<String>) { LOG.with(|l| l.borrow_mut().push(s.into())); }
fn flush(t: &str) { let v = LOG.with(|l| std::mem::take(&mut *l.borrow_mut())); println!("{:<14}{}", t, v.join(" | ")); }

struct D(String);
impl Drop for D { fn drop(&mut self) { log(format!("~{}", self.0)); } }

struct Yield { n: u32 }
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.n == 0 { Poll::Ready(()) } else { self.n -= 1; cx.waker().wake_by_ref(); Poll::Pending }
    }
}
fn yield_n(n: u32) -> Yield { Yield { n } }

fn block_on<F: Future>(f: F) -> (F::Output, u32) {
    let mut f = Box::pin(f);
    let mut cx = Context::from_waker(Waker::noop());
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return (v, polls); }
    }
}

trait Store {
    async fn get(&self, k: &str) -> Option<u32>;
    async fn put(&mut self, k: &str, v: u32);
    async fn incr(&mut self, k: &str) -> u32 {
        let cur = self.get(k).await.unwrap_or(0);
        self.put(k, cur + 1).await;
        cur + 1
    }
}

struct MemStore { data: Vec<(String, u32)>, latency: u32 }
impl Store for MemStore {
    async fn get(&self, k: &str) -> Option<u32> {
        yield_n(self.latency).await;
        self.data.iter().find(|(kk, _)| kk == k).map(|(_, v)| *v)
    }
    async fn put(&mut self, k: &str, v: u32) {
        yield_n(self.latency).await;
        match self.data.iter_mut().find(|(kk, _)| kk == k) {
            Some(e) => e.1 = v,
            None => self.data.push((k.to_string(), v)),
        }
    }
}

async fn use_store<S: Store>(s: &mut S, keys: &[&str]) -> Vec<u32> {
    let mut out = vec![];
    for k in keys { out.push(s.incr(k).await); }
    out
}

fn fib(n: u32) -> Pin<Box<dyn Future<Output = u64>>> {
    Box::pin(async move {
        if n < 2 { return n as u64; }
        yield_n(1).await;
        fib(n - 1).await + fib(n - 2).await
    })
}

async fn with_locals(tag: &str, steps: u32) -> usize {
    let _a = D(format!("{}-a", tag));
    yield_n(steps).await;
    let b = D(format!("{}-b", tag));
    yield_n(steps).await;
    let _c = D(format!("{}-c", tag));
    log(format!("{} finishing", tag));
    b.0.len()
}

async fn borrow_across(v: &mut Vec<String>, extra: &str) -> usize {
    let first = &v[0];
    let l = first.len();
    yield_n(2).await;
    v.push(format!("{}{}", extra, l));
    let joined = v.join("+");
    yield_n(1).await;
    joined.len()
}

struct Select<A, B> { a: Option<Pin<Box<A>>>, b: Option<Pin<Box<B>>> }
enum Either<L, R> { Left(L), Right(R) }
impl<A: Future, B: Future> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = self.a.as_mut().unwrap().as_mut().poll(cx) { self.a = None; self.b = None; return Poll::Ready(Either::Left(v)); }
        if let Poll::Ready(v) = self.b.as_mut().unwrap().as_mut().poll(cx) { self.a = None; self.b = None; return Poll::Ready(Either::Right(v)); }
        Poll::Pending
    }
}

fn main() {
    let mut s = MemStore { data: vec![], latency: 2 };
    let (r, polls) = block_on(use_store(&mut s, &["x", "y", "x", "x", "z", "y"]));
    println!("incr results {:?} polls {} data {:?}", r, polls, s.data);
    let (r2, _) = block_on(async { s.get("x").await.zip(s.get("nope").await.or(Some(0))) });
    println!("get zip {:?}", r2);

    let (f, polls) = block_on(fib(12));
    println!("fib(12)={} polls={}", f, polls);

    let (n, _) = block_on(with_locals("done", 1));
    log(format!("returned {}", n));
    flush("complete:");

    {
        let mut fut = Box::pin(with_locals("cancel", 3));
        let mut cx = Context::from_waker(Waker::noop());
        for _ in 0..5 { let _ = fut.as_mut().poll(&mut cx); }
        log("dropping future");
    }
    flush("cancelled:");

    {
        let fut = with_locals("never", 1);
        log("created but not polled");
        drop(fut);
    }
    flush("unpolled:");

    let mut v = vec!["abc".to_string()];
    let (len, _) = block_on(borrow_across(&mut v, "n"));
    println!("borrow_across len {} v {:?}", len, v);

    let shared = Rc::new(RefCell::new(0));
    let s1 = shared.clone();
    let s2 = shared.clone();
    let sel = Select {
        a: Some(Box::pin(async move { for _ in 0..5 { *s1.borrow_mut() += 1; yield_n(1).await; } let _g = D("slow-guard".into()); yield_n(10).await; "slow" })),
        b: Some(Box::pin(async move { let _g = D("fast-guard".into()); for _ in 0..3 { *s2.borrow_mut() += 10; yield_n(1).await; } 7u8 })),
    };
    let (res, polls) = block_on(sel);
    match res { Either::Left(s) => log(format!("left {}", s)), Either::Right(n) => log(format!("right {}", n)) }
    log(format!("counter {} polls {} rc={}", shared.borrow(), polls, Rc::strong_count(&shared)));
    flush("select:");
}
