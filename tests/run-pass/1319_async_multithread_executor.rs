// A multi-threaded executor: tasks are Arc<Task> holding Mutex<Option<BoxFuture + Send>>,
// Wake re-queues them on a shared queue served by worker threads parked on a Condvar;
// futures yield and wake each other through a shared async channel built on wakers.
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::mpsc;
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread;

type BoxFut = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Shared {
    queue: Mutex<(VecDeque<Arc<Task>>, bool)>,
    cv: Condvar,
    live: AtomicUsize,
}

struct Task {
    fut: Mutex<Option<BoxFut>>,
    shared: Arc<Shared>,
    polls: AtomicUsize,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        let sh = self.shared.clone();
        sh.queue.lock().unwrap().0.push_back(self);
        sh.cv.notify_one();
    }
}

fn spawn(sh: &Arc<Shared>, f: impl Future<Output = ()> + Send + 'static) {
    sh.live.fetch_add(1, Ordering::SeqCst);
    let t = Arc::new(Task { fut: Mutex::new(Some(Box::pin(f))), shared: sh.clone(), polls: AtomicUsize::new(0) });
    t.wake();
}

fn worker(sh: Arc<Shared>) -> usize {
    let mut ran = 0;
    loop {
        let task = {
            let mut g = sh.queue.lock().unwrap();
            loop {
                if let Some(t) = g.0.pop_front() {
                    break t;
                }
                if g.1 {
                    return ran;
                }
                g = sh.cv.wait(g).unwrap();
            }
        };
        let mut slot = task.fut.lock().unwrap();
        if let Some(mut fut) = slot.take() {
            task.polls.fetch_add(1, Ordering::SeqCst);
            let waker = Waker::from(task.clone());
            let mut cx = Context::from_waker(&waker);
            ran += 1;
            if fut.as_mut().poll(&mut cx).is_pending() {
                *slot = Some(fut);
            } else if sh.live.fetch_sub(1, Ordering::SeqCst) == 1 {
                sh.queue.lock().unwrap().1 = true;
                sh.cv.notify_all();
            }
        }
    }
}

// a tiny async channel
struct Chan<T> {
    inner: Mutex<(VecDeque<T>, Option<Waker>, bool)>,
}
impl<T> Chan<T> {
    fn send(&self, v: T) {
        let w = {
            let mut g = self.inner.lock().unwrap();
            g.0.push_back(v);
            g.1.take()
        };
        if let Some(w) = w {
            w.wake();
        }
    }
    fn close(&self) {
        let w = {
            let mut g = self.inner.lock().unwrap();
            g.2 = true;
            g.1.take()
        };
        if let Some(w) = w {
            w.wake();
        }
    }
    fn recv(&self) -> Recv<'_, T> {
        Recv(self)
    }
}
struct Recv<'a, T>(&'a Chan<T>);
impl<T> Future for Recv<'_, T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut g = self.0.inner.lock().unwrap();
        if let Some(v) = g.0.pop_front() {
            Poll::Ready(Some(v))
        } else if g.2 {
            Poll::Ready(None)
        } else {
            g.1 = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

struct YieldNow(bool);
impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn main() {
    let sh = Arc::new(Shared { queue: Mutex::new((VecDeque::new(), false)), cv: Condvar::new(), live: AtomicUsize::new(0) });
    let chan = Arc::new(Chan { inner: Mutex::new((VecDeque::new(), None, false)) });
    let (tx, rx) = mpsc::channel::<String>();
    let producers_left = Arc::new(AtomicUsize::new(3));

    // consumer task
    {
        let chan = chan.clone();
        let tx = tx.clone();
        spawn(&sh, async move {
            let mut total = 0u64;
            let mut n = 0;
            while let Some(v) = chan.recv().await {
                total += v;
                n += 1;
            }
            tx.send(format!("consumer got {} items total {}", n, total)).unwrap();
        });
    }
    // producer tasks
    for p in 0..3u64 {
        let chan = chan.clone();
        let tx = tx.clone();
        let left = producers_left.clone();
        spawn(&sh, async move {
            for i in 0..20u64 {
                chan.send(p * 100 + i);
                if i % 5 == 0 {
                    YieldNow(false).await;
                }
            }
            tx.send(format!("producer {} done", p)).unwrap();
            if left.fetch_sub(1, Ordering::SeqCst) == 1 {
                chan.close();
            }
        });
    }
    drop(tx);
    let workers: Vec<_> = (0..3).map(|_| {
        let sh = sh.clone();
        thread::spawn(move || worker(sh))
    }).collect();
    let ran: usize = workers.into_iter().map(|h| h.join().unwrap()).sum();
    let mut msgs: Vec<String> = rx.iter().collect();
    msgs.sort();
    for m in &msgs {
        println!("{}", m);
    }
    println!("polls >= 5: {} live {}", ran >= 5, sh.live.load(Ordering::SeqCst));
}
