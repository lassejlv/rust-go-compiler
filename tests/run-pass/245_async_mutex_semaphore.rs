// An async Mutex and Semaphore with FIFO waiter queues on a single-threaded executor,
// plus RAII guards that wake the next waiter on drop.
use std::cell::{Cell, RefCell, UnsafeCell};
use std::collections::VecDeque;
use std::future::Future;
use std::ops::{Deref, DerefMut};
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex as StdMutex};
use std::task::{Context, Poll, Wake, Waker};

struct AsyncMutex<T> {
    locked: Cell<bool>,
    waiters: RefCell<VecDeque<Waker>>,
    value: UnsafeCell<T>,
}

struct Guard<'a, T> {
    m: &'a AsyncMutex<T>,
}

impl<T> AsyncMutex<T> {
    fn new(v: T) -> Self {
        AsyncMutex { locked: Cell::new(false), waiters: RefCell::new(VecDeque::new()), value: UnsafeCell::new(v) }
    }
    fn lock(&self) -> Lock<'_, T> {
        Lock { m: self, queued: false }
    }
}

struct Lock<'a, T> {
    m: &'a AsyncMutex<T>,
    queued: bool,
}

impl<'a, T> Future for Lock<'a, T> {
    type Output = Guard<'a, T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Guard<'a, T>> {
        if !self.m.locked.get() {
            self.m.locked.set(true);
            return Poll::Ready(Guard { m: self.m });
        }
        if !self.queued {
            self.m.waiters.borrow_mut().push_back(cx.waker().clone());
            self.queued = true;
        }
        Poll::Pending
    }
}

impl<T> Deref for Guard<'_, T> {
    type Target = T;
    fn deref(&self) -> &T {
        unsafe { &*self.m.value.get() }
    }
}
impl<T> DerefMut for Guard<'_, T> {
    fn deref_mut(&mut self) -> &mut T {
        unsafe { &mut *self.m.value.get() }
    }
}
impl<T> Drop for Guard<'_, T> {
    fn drop(&mut self) {
        self.m.locked.set(false);
        if let Some(w) = self.m.waiters.borrow_mut().pop_front() {
            w.wake();
        }
    }
}

struct Semaphore {
    permits: Cell<usize>,
    waiters: RefCell<VecDeque<Waker>>,
}

struct Permit<'a>(&'a Semaphore);
impl Drop for Permit<'_> {
    fn drop(&mut self) {
        self.0.permits.set(self.0.permits.get() + 1);
        if let Some(w) = self.0.waiters.borrow_mut().pop_front() {
            w.wake();
        }
    }
}

impl Semaphore {
    async fn acquire(&self) -> Permit<'_> {
        let mut queued = false;
        std::future::poll_fn(|cx| {
            if self.permits.get() > 0 {
                self.permits.set(self.permits.get() - 1);
                Poll::Ready(())
            } else {
                if !queued {
                    self.waiters.borrow_mut().push_back(cx.waker().clone());
                    queued = true;
                } else {
                    // re-queue if woken but lost the race
                    self.waiters.borrow_mut().push_back(cx.waker().clone());
                }
                Poll::Pending
            }
        })
        .await;
        Permit(self)
    }
}

async fn yield_now() {
    let mut done = false;
    std::future::poll_fn(|cx| {
        if done {
            Poll::Ready(())
        } else {
            done = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    })
    .await
}

struct W(usize, Arc<StdMutex<VecDeque<usize>>>);
impl Wake for W {
    fn wake(self: Arc<Self>) {
        self.1.lock().unwrap().push_back(self.0);
    }
}

fn run_all(tasks: Vec<Pin<Box<dyn Future<Output = ()> + '_>>>) -> usize {
    let q = Arc::new(StdMutex::new((0..tasks.len()).collect::<VecDeque<_>>()));
    let mut ts: Vec<_> = tasks.into_iter().map(Some).collect();
    let mut polls = 0;
    loop {
        let n = q.lock().unwrap().pop_front();
        let Some(i) = n else { break };
        if let Some(t) = ts[i].as_mut() {
            polls += 1;
            let w = Waker::from(Arc::new(W(i, q.clone())));
            if t.as_mut().poll(&mut Context::from_waker(&w)).is_ready() {
                ts[i] = None;
            }
        }
    }
    polls
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let m = AsyncMutex::new(Vec::<String>::new());
    let mut tasks: Vec<Pin<Box<dyn Future<Output = ()> + '_>>> = vec![];
    for id in 0..3 {
        let m = &m;
        let log = log.clone();
        tasks.push(Box::pin(async move {
            for round in 0..2 {
                let mut g = m.lock().await;
                log.borrow_mut().push(format!("task{} locked (round {})", id, round));
                g.push(format!("{}.{}", id, round));
                yield_now().await; // hold the lock across a yield
                log.borrow_mut().push(format!("task{} unlocking", id));
                drop(g);
                yield_now().await;
            }
        }));
    }
    let polls = run_all(tasks);
    for l in log.borrow().iter() {
        println!("{}", l);
    }
    println!("data: {:?} polls={}", unsafe { &*m.value.get() }, polls);

    let sem = Semaphore { permits: Cell::new(2), waiters: RefCell::new(VecDeque::new()) };
    let active = Cell::new(0);
    let max_active = Cell::new(0);
    let order = RefCell::new(vec![]);
    let mut tasks: Vec<Pin<Box<dyn Future<Output = ()> + '_>>> = vec![];
    for id in 0..5 {
        let (sem, active, max_active, order) = (&sem, &active, &max_active, &order);
        tasks.push(Box::pin(async move {
            let _p = sem.acquire().await;
            active.set(active.get() + 1);
            max_active.set(max_active.get().max(active.get()));
            order.borrow_mut().push(format!("start{}", id));
            for _ in 0..id % 3 + 1 {
                yield_now().await;
            }
            order.borrow_mut().push(format!("end{}", id));
            active.set(active.get() - 1);
        }));
    }
    let polls = run_all(tasks);
    println!("{}", order.borrow().join(" "));
    println!("max concurrent={} permits left={} polls={}", max_active.get(), sem.permits.get(), polls);
}
