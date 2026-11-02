use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct CountingWaker {
    wakes: AtomicUsize,
}

impl Wake for CountingWaker {
    fn wake(self: Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
    }
}

struct YieldOnce {
    yielded: bool,
}

impl Future for YieldOnce {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.yielded {
            Poll::Ready(())
        } else {
            self.yielded = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn yield_now() -> YieldOnce {
    YieldOnce { yielded: false }
}

fn run<F: Future>(fut: F) -> (F::Output, usize, usize) {
    let cw = Arc::new(CountingWaker { wakes: AtomicUsize::new(0) });
    let waker = Waker::from(cw.clone());
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    let mut polls = 0;
    loop {
        polls += 1;
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => return (v, polls, cw.wakes.load(Ordering::SeqCst)),
            Poll::Pending => {}
        }
    }
}

async fn produce(tag: &str, n: usize) -> Vec<String> {
    let mut out = Vec::new();
    for i in 0..n {
        out.push(format!("{}{}", tag, i));
        yield_now().await;
    }
    out
}

fn main() {
    let (v, polls, wakes) = run(produce("x", 3));
    println!("{:?} polls={} wakes={}", v, polls, wakes);

    let prefix = String::from("item-");
    let mut counter = 10;
    let fut = async move {
        let mut acc = Vec::new();
        for _ in 0..2 {
            yield_now().await;
            counter += 5;
            acc.push(format!("{}{}", prefix, counter));
        }
        acc
    };
    let (v, polls, wakes) = run(fut);
    println!("{:?} polls={} wakes={}", v, polls, wakes);
    println!("outer counter still {}", counter);

    let data = vec![3, 1, 2];
    let borrowed = async {
        let s: i32 = data.iter().sum();
        yield_now().await;
        s * 100
    };
    let (r, polls, _) = run(borrowed);
    println!("sum*100 = {} polls={} data={:?}", r, polls, data);

    let nested = async {
        let a = async { 1 }.await;
        let b = async move { a + 1 }.await;
        let c = produce("n", 1).await;
        format!("{} {} {:?}", a, b, c)
    };
    let (s, polls, wakes) = run(nested);
    println!("{} polls={} wakes={}", s, polls, wakes);

    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut y = yield_now();
    println!("{:?}", Pin::new(&mut y).poll(&mut cx));
    println!("{:?}", Pin::new(&mut y).poll(&mut cx));
}
