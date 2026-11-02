// Single-threaded async channels built on Rc<RefCell<VecDeque>> + stored Wakers, with an
// executor that runs local (non-Send) tasks.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

struct Shared<T> {
    queue: VecDeque<T>,
    cap: usize,
    recv_waker: Option<Waker>,
    send_wakers: Vec<Waker>,
    senders: usize,
}

struct Sender<T>(Rc<RefCell<Shared<T>>>);
struct Receiver<T>(Rc<RefCell<Shared<T>>>);

fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let s = Rc::new(RefCell::new(Shared { queue: VecDeque::new(), cap, recv_waker: None, send_wakers: vec![], senders: 1 }));
    (Sender(s.clone()), Receiver(s))
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        self.0.borrow_mut().senders += 1;
        Sender(self.0.clone())
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut s = self.0.borrow_mut();
        s.senders -= 1;
        if s.senders == 0 {
            if let Some(w) = s.recv_waker.take() {
                w.wake();
            }
        }
    }
}

struct SendFut<'a, T> {
    tx: &'a Sender<T>,
    item: Option<T>,
}

impl<T> Future for SendFut<'_, T> {
    type Output = ();
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let this = unsafe { self.get_unchecked_mut() };
        let mut s = this.tx.0.borrow_mut();
        if s.queue.len() >= s.cap {
            s.send_wakers.push(cx.waker().clone());
            return Poll::Pending;
        }
        s.queue.push_back(this.item.take().unwrap());
        if let Some(w) = s.recv_waker.take() {
            w.wake();
        }
        Poll::Ready(())
    }
}

impl<T> Sender<T> {
    fn send(&self, item: T) -> SendFut<'_, T> {
        SendFut { tx: self, item: Some(item) }
    }
}

struct RecvFut<'a, T> {
    rx: &'a Receiver<T>,
}

impl<T> Future for RecvFut<'_, T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut s = self.rx.0.borrow_mut();
        if let Some(v) = s.queue.pop_front() {
            for w in s.send_wakers.drain(..) {
                w.wake();
            }
            Poll::Ready(Some(v))
        } else if s.senders == 0 {
            Poll::Ready(None)
        } else {
            s.recv_waker = Some(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T> Receiver<T> {
    fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { rx: self }
    }
}

// Local executor: tasks are identified by index; wakers push the index to a shared ready list.
struct IdWaker {
    id: usize,
    ready: Arc<Mutex<Vec<usize>>>,
}
impl Wake for IdWaker {
    fn wake(self: Arc<Self>) {
        let mut r = self.ready.lock().unwrap();
        if !r.contains(&self.id) {
            r.push(self.id);
        }
    }
}

fn run_local(tasks: Vec<Pin<Box<dyn Future<Output = ()>>>>) -> usize {
    let ready = Arc::new(Mutex::new((0..tasks.len()).collect::<Vec<_>>()));
    let mut slots: Vec<Option<Pin<Box<dyn Future<Output = ()>>>>> = tasks.into_iter().map(Some).collect();
    let mut polls = 0;
    loop {
        let batch: Vec<usize> = std::mem::take(&mut *ready.lock().unwrap());
        if batch.is_empty() {
            break;
        }
        for id in batch {
            if let Some(f) = slots[id].as_mut() {
                let w = Waker::from(Arc::new(IdWaker { id, ready: ready.clone() }));
                polls += 1;
                if f.as_mut().poll(&mut Context::from_waker(&w)).is_ready() {
                    slots[id] = None;
                }
            }
        }
    }
    polls
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::<String>::new()));
    let (tx, rx) = channel::<u32>(2);
    let (otx, orx) = channel::<String>(8);

    let mut tasks: Vec<Pin<Box<dyn Future<Output = ()>>>> = vec![];
    for p in 0..2u32 {
        let tx = tx.clone();
        let log = log.clone();
        tasks.push(Box::pin(async move {
            for i in 0..4 {
                let v = p * 100 + i;
                tx.send(v).await;
                log.borrow_mut().push(format!("producer{} sent {}", p, v));
            }
            log.borrow_mut().push(format!("producer{} done", p));
        }));
    }
    drop(tx);
    {
        let log = log.clone();
        tasks.push(Box::pin(async move {
            let mut sum = 0;
            let mut n = 0;
            while let Some(v) = rx.recv().await {
                sum += v;
                n += 1;
                log.borrow_mut().push(format!("consumer got {}", v));
                if v % 2 == 1 {
                    otx.send(format!("odd:{}", v)).await;
                }
            }
            log.borrow_mut().push(format!("consumer closed: n={} sum={}", n, sum));
        }));
    }
    {
        let log = log.clone();
        tasks.push(Box::pin(async move {
            let mut all = vec![];
            while let Some(s) = orx.recv().await {
                all.push(s);
            }
            log.borrow_mut().push(format!("odds: {}", all.join(",")));
        }));
    }
    let polls = run_local(tasks);
    for l in log.borrow().iter() {
        println!("{}", l);
    }
    println!("polls={}", polls);
}
