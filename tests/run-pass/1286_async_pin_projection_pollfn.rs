// Pin projection by hand (map_unchecked_mut), Pin::set/get_mut/into_inner, poll_fn, ready!,
// IntoFuture, std::future::{ready, pending}.
use std::future::{Future, IntoFuture};
use std::pin::Pin;
use std::task::{ready, Context, Poll, Waker};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

struct Delay {
    left: u32,
}
impl Future for Delay {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u32> {
        if self.left == 0 { Poll::Ready(7) } else { self.left -= 1; Poll::Pending }
    }
}

// A !Unpin future wrapper with a projection to the inner future.
struct Timed<F> {
    inner: F,
    polls: u32,
    _pin: std::marker::PhantomPinned,
}
impl<F: Future> Future for Timed<F> {
    type Output = (F::Output, u32);
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = unsafe { self.get_unchecked_mut() };
        this.polls += 1;
        let inner = unsafe { Pin::new_unchecked(&mut this.inner) };
        let v = ready!(inner.poll(cx));
        Poll::Ready((v, this.polls))
    }
}

struct AddOne<F>(F);
impl<F: Future<Output = u32>> Future for AddOne<F> {
    type Output = u32;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        let inner = unsafe { self.map_unchecked_mut(|s| &mut s.0) };
        inner.poll(cx).map(|v| v + 1)
    }
}

struct Request {
    path: &'static str,
}
impl IntoFuture for Request {
    type Output = String;
    type IntoFuture = Pin<Box<dyn Future<Output = String>>>;
    fn into_future(self) -> Self::IntoFuture {
        Box::pin(async move {
            Delay { left: 2 }.await;
            format!("GET {} -> 200", self.path)
        })
    }
}

fn main() {
    let t = Timed { inner: Delay { left: 3 }, polls: 0, _pin: std::marker::PhantomPinned };
    println!("{:?}", block_on(t));
    println!("{}", block_on(AddOne(AddOne(Delay { left: 1 }))));

    let mut counter = 0;
    let pf = std::future::poll_fn(|cx: &mut Context<'_>| {
        counter += 1;
        if counter < 4 {
            cx.waker().wake_by_ref();
            Poll::Pending
        } else {
            Poll::Ready(counter * 100)
        }
    });
    println!("poll_fn {}", block_on(pf));

    println!("{}", block_on(Request { path: "/index" }.into_future()));
    println!("{}", block_on(async { Request { path: "/a" }.await + "!" }));
    println!("{}", block_on(std::future::ready(5)));

    // pending never completes: poll it a few times manually
    let mut cx = Context::from_waker(Waker::noop());
    let mut p = std::pin::pin!(std::future::pending::<u8>());
    println!("pending: {:?} {:?}", p.as_mut().poll(&mut cx), p.as_mut().poll(&mut cx));

    // Pin::set replaces the pinned value (dropping the old one)
    let mut slot = Box::pin(Delay { left: 5 });
    let _ = slot.as_mut().poll(&mut cx);
    println!("left before set {}", slot.left);
    slot.set(Delay { left: 0 });
    println!("{:?}", slot.as_mut().poll(&mut cx));
    let mut d = Delay { left: 2 };
    let mut pd = Pin::new(&mut d);
    let _ = pd.as_mut().poll(&mut cx);
    pd.get_mut().left += 10;
    println!("unpin get_mut {}", d.left);
    let boxed = Pin::into_inner(Box::pin(Delay { left: 9 }));
    println!("into_inner {}", boxed.left);
    let pr = Pin::new(&5);
    println!("pinned ref {} {}", *pr, pr.get_ref());
    let ps = Box::pin(String::from("pinned string"));
    println!("{} {}", ps.len(), ps.as_ref().get_ref());
}
