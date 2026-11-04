// Futures suspended on one thread and resumed on another (work-stealing style), with
// thread-local state observed from inside the future, plus many concurrently pending futures.
use std::cell::Cell;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::{Context, Poll, Waker};
use std::thread;

thread_local! {
    static WORKER: Cell<u32> = const { Cell::new(0) };
}

struct Suspend(bool);
impl Future for Suspend {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 { Poll::Ready(()) } else { self.0 = true; Poll::Pending }
    }
}

async fn job(id: u32) -> Vec<u32> {
    let mut seen = Vec::new();
    let mut acc = id;
    for step in 0..3 {
        seen.push(WORKER.with(|w| w.get()));
        acc = acc.wrapping_mul(31).wrapping_add(step);
        Suspend(false).await;
    }
    seen.push(WORKER.get());
    seen.push(acc % 1000);
    seen
}

type BoxFut = Pin<Box<dyn Future<Output = Vec<u32>> + Send>>;

fn main() {
    // stage pipeline: each worker polls a future once and passes it on
    let n_workers = 4;
    let mut senders = Vec::new();
    let mut receivers = Vec::new();
    for _ in 0..=n_workers {
        let (tx, rx) = mpsc::channel::<(u32, BoxFut)>();
        senders.push(tx);
        receivers.push(Some(rx));
    }
    let (done_tx, done_rx) = mpsc::channel::<(u32, Vec<u32>)>();
    let mut handles = Vec::new();
    for w in 0..n_workers {
        let rx = receivers[w].take().unwrap();
        let next = senders[w + 1].clone();
        let done = done_tx.clone();
        handles.push(thread::spawn(move || {
            WORKER.set(w as u32 + 1);
            let mut cx = Context::from_waker(Waker::noop());
            for (id, mut fut) in rx {
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(v) => done.send((id, v)).unwrap(),
                    Poll::Pending => next.send((id, fut)).unwrap(),
                }
            }
        }));
    }
    drop(done_tx);
    let last_rx = receivers[n_workers].take().unwrap();
    for id in 0..6u32 {
        senders[0].send((id, Box::pin(job(id)))).unwrap();
    }
    drop(senders);
    for h in handles {
        h.join().unwrap();
    }
    let mut results: Vec<_> = done_rx.iter().collect();
    results.sort();
    for r in &results {
        println!("{:?}", r);
    }
    println!("left over {}", last_rx.try_iter().count());

    // many pending futures at once
    let mut cx = Context::from_waker(Waker::noop());
    let mut futs: Vec<Pin<Box<dyn Future<Output = u64>>>> = (0..3000u64)
        .map(|i| -> Pin<Box<dyn Future<Output = u64>>> {
            Box::pin(async move {
                let a = i * 2;
                Suspend(false).await;
                a + 1
            })
        })
        .collect();
    let mut pending = 0;
    for f in futs.iter_mut() {
        if f.as_mut().poll(&mut cx).is_pending() {
            pending += 1;
        }
    }
    let mut sum = 0;
    for f in futs.iter_mut() {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            sum += v;
        }
    }
    println!("pending {} sum {}", pending, sum);
    drop(futs);
    // drop many futures that are suspended mid-way
    let mut half: Vec<_> = (0..2000).map(|i| Box::pin(async move { Suspend(false).await; i })).collect();
    for f in half.iter_mut() {
        let _ = f.as_mut().poll(&mut cx);
    }
    drop(half);
    println!("dropped suspended");
}
