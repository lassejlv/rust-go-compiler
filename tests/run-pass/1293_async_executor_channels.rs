// A single-threaded executor with spawn(), JoinHandle futures, a oneshot channel and
// an async mutex-like semaphore built from Rc<RefCell<..>> and stored wakers.
use std::cell::{Cell, RefCell};
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

type BoxFut = Pin<Box<dyn Future<Output = ()>>>;

struct Task {
    fut: RefCell<Option<BoxFut>>,
}

struct Notify {
    id: usize,
    ready: Arc<Mutex<VecDeque<usize>>>,
}
impl Wake for Notify {
    fn wake(self: Arc<Self>) {
        self.ready.lock().unwrap().push_back(self.id);
    }
}

#[derive(Clone)]
struct Executor {
    tasks: Rc<RefCell<Vec<Rc<Task>>>>,
    ready: Arc<Mutex<VecDeque<usize>>>,
}

struct JoinState<T> {
    val: Option<T>,
    waker: Option<Waker>,
}
struct JoinHandle<T>(Rc<RefCell<JoinState<T>>>);
impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut st = self.0.borrow_mut();
        match st.val.take() {
            Some(v) => Poll::Ready(v),
            None => {
                st.waker = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Executor {
    fn new() -> Self {
        Executor { tasks: Rc::new(RefCell::new(Vec::new())), ready: Arc::new(Mutex::new(VecDeque::new())) }
    }
    fn spawn<T: 'static>(&self, f: impl Future<Output = T> + 'static) -> JoinHandle<T> {
        let st = Rc::new(RefCell::new(JoinState { val: None, waker: None }));
        let st2 = st.clone();
        let wrapped = async move {
            let v = f.await;
            let w = {
                let mut s = st2.borrow_mut();
                s.val = Some(v);
                s.waker.take()
            };
            if let Some(w) = w {
                w.wake();
            }
        };
        let mut tasks = self.tasks.borrow_mut();
        let id = tasks.len();
        tasks.push(Rc::new(Task { fut: RefCell::new(Some(Box::pin(wrapped))) }));
        self.ready.lock().unwrap().push_back(id);
        JoinHandle(st)
    }
    fn run(&self) -> usize {
        let mut polls = 0;
        loop {
            let next = self.ready.lock().unwrap().pop_front();
            let Some(id) = next else { break };
            let task = self.tasks.borrow()[id].clone();
            let mut slot = task.fut.borrow_mut();
            if let Some(fut) = slot.as_mut() {
                let waker = Waker::from(Arc::new(Notify { id, ready: self.ready.clone() }));
                let mut cx = Context::from_waker(&waker);
                polls += 1;
                if fut.as_mut().poll(&mut cx).is_ready() {
                    *slot = None;
                }
            }
        }
        polls
    }
}

struct Semaphore {
    permits: Cell<usize>,
    waiters: RefCell<VecDeque<Waker>>,
}
struct Acquire<'a>(&'a Semaphore);
impl<'a> Future for Acquire<'a> {
    type Output = Permit<'a>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Permit<'a>> {
        let s = self.0;
        if s.permits.get() > 0 {
            s.permits.set(s.permits.get() - 1);
            Poll::Ready(Permit(s))
        } else {
            s.waiters.borrow_mut().push_back(cx.waker().clone());
            Poll::Pending
        }
    }
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

struct YieldNow(bool);
impl Future for YieldNow {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            return Poll::Ready(());
        }
        self.0 = true;
        cx.waker().wake_by_ref();
        Poll::Pending
    }
}

fn main() {
    let ex = Executor::new();
    let log = Rc::new(RefCell::new(Vec::new()));
    let sem: &'static Semaphore = Box::leak(Box::new(Semaphore { permits: Cell::new(2), waiters: RefCell::new(VecDeque::new()) }));
    let mut handles = Vec::new();
    for i in 0..5 {
        let log = log.clone();
        handles.push(ex.spawn(async move {
            let _p = Acquire(sem).await;
            log.borrow_mut().push(format!("task {} acquired (free {})", i, sem.permits.get()));
            YieldNow(false).await;
            YieldNow(false).await;
            log.borrow_mut().push(format!("task {} releasing", i));
            i * 11
        }));
    }
    let ex2 = ex.clone();
    let log2 = log.clone();
    let collector = ex.spawn(async move {
        let mut total = 0;
        for h in handles {
            total += h.await;
        }
        let inner = ex2.spawn(async { YieldNow(false).await; "nested" });
        let s = inner.await;
        log2.borrow_mut().push(format!("collector total {} {}", total, s));
        total
    });
    let polls = ex.run();
    for l in log.borrow().iter() {
        println!("{}", l);
    }
    println!("polls {} permits {} tasks {}", polls, sem.permits.get(), ex.tasks.borrow().len());
    let mut c = collector;
    let mut cx = Context::from_waker(Waker::noop());
    println!("{:?}", Pin::new(&mut c).poll(&mut cx));
}
