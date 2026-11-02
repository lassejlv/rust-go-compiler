// join_all / join2 / try_join implemented by hand over pinned boxed futures.
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(fut: F) -> (F::Output, u32) {
    let waker = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return (v, polls);
        }
    }
}

struct Delay {
    ticks: u32,
    label: &'static str,
}
impl Future for Delay {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<&'static str> {
        if self.ticks == 0 {
            Poll::Ready(self.label)
        } else {
            self.ticks -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn delay(ticks: u32, label: &'static str) -> Delay {
    Delay { ticks, label }
}

enum MaybeDone<F: Future> {
    Pending(Pin<Box<F>>),
    Done(F::Output),
    Taken,
}

struct JoinAll<F: Future> {
    items: Vec<MaybeDone<F>>,
}

fn join_all<I>(it: I) -> JoinAll<I::Item>
where
    I: IntoIterator,
    I::Item: Future,
{
    JoinAll { items: it.into_iter().map(|f| MaybeDone::Pending(Box::pin(f))).collect() }
}

impl<F: Future> Future for JoinAll<F>
where
    F::Output: Unpin,
{
    type Output = Vec<F::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let mut all_done = true;
        for it in self.items.iter_mut() {
            if let MaybeDone::Pending(f) = it {
                match f.as_mut().poll(cx) {
                    Poll::Ready(v) => *it = MaybeDone::Done(v),
                    Poll::Pending => all_done = false,
                }
            }
        }
        if !all_done {
            return Poll::Pending;
        }
        let out = self
            .items
            .iter_mut()
            .map(|m| match std::mem::replace(m, MaybeDone::Taken) {
                MaybeDone::Done(v) => v,
                _ => unreachable!(),
            })
            .collect();
        Poll::Ready(out)
    }
}

// join two heterogeneous futures
async fn join2<A: Future, B: Future>(a: A, b: B) -> (A::Output, B::Output) {
    let mut a = Box::pin(a);
    let mut b = Box::pin(b);
    let mut ra = None;
    let mut rb = None;
    std::future::poll_fn(|cx| {
        if ra.is_none() {
            if let Poll::Ready(v) = a.as_mut().poll(cx) {
                ra = Some(v);
            }
        }
        if rb.is_none() {
            if let Poll::Ready(v) = b.as_mut().poll(cx) {
                rb = Some(v);
            }
        }
        if ra.is_some() && rb.is_some() {
            Poll::Ready(())
        } else {
            Poll::Pending
        }
    })
    .await;
    (ra.unwrap(), rb.unwrap())
}

// try_join_all: short-circuit on first error (in poll order)
async fn try_join_all<T, E, F>(futs: Vec<F>) -> Result<Vec<T>, E>
where
    F: Future<Output = Result<T, E>>,
{
    let mut pinned: Vec<Pin<Box<F>>> = futs.into_iter().map(Box::pin).collect();
    let mut results: Vec<Option<T>> = (0..pinned.len()).map(|_| None).collect();
    std::future::poll_fn(move |cx| {
        let mut pending = false;
        for (i, f) in pinned.iter_mut().enumerate() {
            if results[i].is_some() {
                continue;
            }
            match f.as_mut().poll(cx) {
                Poll::Ready(Ok(v)) => results[i] = Some(v),
                Poll::Ready(Err(e)) => return Poll::Ready(Err(e)),
                Poll::Pending => pending = true,
            }
        }
        if pending {
            Poll::Pending
        } else {
            Poll::Ready(Ok(results.iter_mut().map(|r| r.take().unwrap()).collect()))
        }
    })
    .await
}

async fn fetch(id: u32) -> Result<String, String> {
    delay(id % 4, "x").await;
    if id == 13 {
        Err(format!("fetch {} failed", id))
    } else {
        Ok(format!("item{}", id))
    }
}

async fn compute(n: u64) -> u64 {
    let mut acc = 0;
    for i in 0..n {
        delay(1, "").await;
        acc += i * i;
    }
    acc
}

fn main() {
    let (labels, polls) = block_on(join_all(vec![delay(3, "c"), delay(0, "a"), delay(5, "e"), delay(1, "b")]));
    println!("{:?} polls={}", labels, polls);

    let (sums, polls) = block_on(join_all((1..=4).map(compute)));
    println!("{:?} polls={}", sums, polls);

    let ((x, y), polls) = block_on(join2(compute(5), async {
        let a = delay(2, "left").await;
        let b = delay(1, "right").await;
        format!("{}+{}", a, b)
    }));
    println!("{} {:?} polls={}", x, y, polls);

    let (r, polls) = block_on(try_join_all(vec![fetch(1), fetch(2), fetch(7)]));
    println!("{:?} polls={}", r, polls);
    let (r, polls) = block_on(try_join_all(vec![fetch(1), fetch(13), fetch(7)]));
    println!("{:?} polls={}", r, polls);

    let (nested, polls) = block_on(async {
        let groups = vec![vec![1u64, 2], vec![3], vec![]];
        let mut out = vec![];
        for g in groups {
            let v = join_all(g.into_iter().map(compute)).await;
            out.push(v.iter().sum::<u64>());
        }
        out
    });
    println!("{:?} polls={}", nested, polls);
    let (empty, polls) = block_on(join_all(Vec::<Delay>::new()));
    println!("{:?} polls={}", empty, polls);
}
