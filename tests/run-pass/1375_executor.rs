// A single-threaded async runtime written from scratch: tasks as
// Pin<Box<dyn Future>>, wakers built with std::task::Wake on Arc, a virtual
// clock with a timer heap, oneshot and mpsc-style channels, a mutex-like
// async semaphore, join/select combinators, and a typed event bus keyed by
// TypeId whose handlers spawn tasks. Output is deterministic.
use std::any::{Any, TypeId};
use std::cell::{Cell, RefCell};
use std::cmp::Reverse;
use std::collections::{BTreeMap, BinaryHeap, HashMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

type TaskId = usize;

struct TaskWaker {
    id: TaskId,
    queue: Arc<Mutex<VecDeque<TaskId>>>,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        let mut q = self.queue.lock().unwrap();
        if !q.contains(&self.id) {
            q.push_back(self.id);
        }
    }
}

#[derive(Default)]
struct Shared {
    now: Cell<u64>,
    timers: RefCell<BinaryHeap<Reverse<(u64, u64, usize)>>>, // (deadline, seq, waker slot)
    timer_wakers: RefCell<HashMap<usize, Waker>>,
    seq: Cell<u64>,
    log: RefCell<Vec<String>>,
    spawned: RefCell<Vec<Pin<Box<dyn Future<Output = ()>>>>>,
}

thread_local! {
    static RT: RefCell<Option<Rc<Shared>>> = RefCell::new(None);
}

fn rt() -> Rc<Shared> {
    RT.with(|r| r.borrow().clone().expect("no runtime"))
}

fn log(msg: String) {
    let r = rt();
    let t = r.now.get();
    r.log.borrow_mut().push(format!("[t={:>3}] {}", t, msg));
}

fn spawn(f: impl Future<Output = ()> + 'static) {
    rt().spawned.borrow_mut().push(Box::pin(f));
}

struct Sleep {
    deadline: u64,
    slot: Option<usize>,
}

fn sleep(dt: u64) -> Sleep {
    Sleep { deadline: rt().now.get() + dt, slot: None }
}

impl Future for Sleep {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        let r = rt();
        if r.now.get() >= self.deadline {
            return Poll::Ready(());
        }
        let slot = match self.slot {
            Some(s) => s,
            None => {
                let s = r.seq.get() as usize;
                r.seq.set(r.seq.get() + 1);
                r.timers.borrow_mut().push(Reverse((self.deadline, s as u64, s)));
                self.slot = Some(s);
                s
            }
        };
        r.timer_wakers.borrow_mut().insert(slot, cx.waker().clone());
        Poll::Pending
    }
}

// ---------------- channels ----------------
struct ChanState<T> {
    buf: VecDeque<T>,
    cap: usize,
    closed: bool,
    recv_waker: Option<Waker>,
    send_wakers: Vec<Waker>,
}

struct Sender<T> {
    st: Rc<RefCell<ChanState<T>>>,
}
struct Receiver<T> {
    st: Rc<RefCell<ChanState<T>>>,
}

fn channel<T>(cap: usize) -> (Sender<T>, Receiver<T>) {
    let st = Rc::new(RefCell::new(ChanState { buf: VecDeque::new(), cap, closed: false, recv_waker: None, send_wakers: vec![] }));
    (Sender { st: st.clone() }, Receiver { st })
}

impl<T> Clone for Sender<T> {
    fn clone(&self) -> Self {
        Sender { st: self.st.clone() }
    }
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        // the last sender closes the channel (the receiver holds one more Rc)
        if Rc::strong_count(&self.st) == 2 {
            let mut s = self.st.borrow_mut();
            s.closed = true;
            if let Some(w) = s.recv_waker.take() {
                w.wake();
            }
        }
    }
}

struct SendFut<'a, T> {
    s: &'a Sender<T>,
    item: Option<T>,
}

impl<'a, T: Unpin> Future for SendFut<'a, T> {
    type Output = Result<(), T>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Result<(), T>> {
        let st = self.s.st.clone();
        let mut s = st.borrow_mut();
        if s.buf.len() < s.cap {
            let item = self.item.take().unwrap();
            s.buf.push_back(item);
            if let Some(w) = s.recv_waker.take() {
                w.wake();
            }
            Poll::Ready(Ok(()))
        } else {
            s.send_wakers.push(cx.waker().clone());
            Poll::Pending
        }
    }
}

impl<T: Unpin> Sender<T> {
    fn send(&self, item: T) -> SendFut<'_, T> {
        SendFut { s: self, item: Some(item) }
    }
}

struct RecvFut<'a, T> {
    r: &'a Receiver<T>,
}

impl<'a, T> Future for RecvFut<'a, T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let mut s = self.r.st.borrow_mut();
        if let Some(v) = s.buf.pop_front() {
            for w in s.send_wakers.drain(..) {
                w.wake();
            }
            return Poll::Ready(Some(v));
        }
        if s.closed {
            return Poll::Ready(None);
        }
        s.recv_waker = Some(cx.waker().clone());
        Poll::Pending
    }
}

impl<T> Receiver<T> {
    fn recv(&self) -> RecvFut<'_, T> {
        RecvFut { r: self }
    }
}

// ---------------- semaphore ----------------
#[derive(Clone)]
struct Semaphore {
    inner: Rc<RefCell<(usize, VecDeque<Waker>)>>,
}

struct Acquire {
    sem: Semaphore,
    queued: bool,
}

struct Permit {
    sem: Semaphore,
}

impl Semaphore {
    fn new(n: usize) -> Self {
        Semaphore { inner: Rc::new(RefCell::new((n, VecDeque::new()))) }
    }
    fn acquire(&self) -> Acquire {
        Acquire { sem: self.clone(), queued: false }
    }
}

impl Future for Acquire {
    type Output = Permit;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Permit> {
        let sem = self.sem.clone();
        let mut g = sem.inner.borrow_mut();
        if g.0 > 0 {
            g.0 -= 1;
            return Poll::Ready(Permit { sem: self.sem.clone() });
        }
        if !self.queued {
            g.1.push_back(cx.waker().clone());
            self.queued = true;
        } else {
            g.1.push_back(cx.waker().clone());
        }
        Poll::Pending
    }
}

impl Drop for Permit {
    fn drop(&mut self) {
        let mut g = self.sem.inner.borrow_mut();
        g.0 += 1;
        if let Some(w) = g.1.pop_front() {
            w.wake();
        }
    }
}

// ---------------- combinators ----------------
struct Join2<A: Future, B: Future> {
    a: Pin<Box<A>>,
    b: Pin<Box<B>>,
    ra: Option<A::Output>,
    rb: Option<B::Output>,
}

impl<A: Future, B: Future> Future for Join2<A, B>
where
    A::Output: Unpin,
    B::Output: Unpin,
{
    type Output = (A::Output, B::Output);
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.ra.is_none() {
            if let Poll::Ready(v) = this.a.as_mut().poll(cx) {
                this.ra = Some(v);
            }
        }
        if this.rb.is_none() {
            if let Poll::Ready(v) = this.b.as_mut().poll(cx) {
                this.rb = Some(v);
            }
        }
        if this.ra.is_some() && this.rb.is_some() {
            Poll::Ready((this.ra.take().unwrap(), this.rb.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

fn join2<A: Future, B: Future>(a: A, b: B) -> Join2<A, B> {
    Join2 { a: Box::pin(a), b: Box::pin(b), ra: None, rb: None }
}

enum Either<L, R> {
    Left(L),
    Right(R),
}

struct Select<A, B> {
    a: Pin<Box<A>>,
    b: Pin<Box<B>>,
}

impl<A: Future, B: Future> Future for Select<A, B> {
    type Output = Either<A::Output, B::Output>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if let Poll::Ready(v) = self.a.as_mut().poll(cx) {
            return Poll::Ready(Either::Left(v));
        }
        if let Poll::Ready(v) = self.b.as_mut().poll(cx) {
            return Poll::Ready(Either::Right(v));
        }
        Poll::Pending
    }
}

fn select<A: Future, B: Future>(a: A, b: B) -> Select<A, B> {
    Select { a: Box::pin(a), b: Box::pin(b) }
}

// ---------------- executor ----------------
fn run(main: impl Future<Output = ()> + 'static) -> Vec<String> {
    let shared = Rc::new(Shared::default());
    RT.with(|r| *r.borrow_mut() = Some(shared.clone()));
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let mut tasks: BTreeMap<TaskId, Pin<Box<dyn Future<Output = ()>>>> = BTreeMap::new();
    let mut next_id = 0;
    shared.spawned.borrow_mut().push(Box::pin(main));
    let mut polls = 0;
    loop {
        // adopt newly spawned tasks
        let new: Vec<_> = shared.spawned.borrow_mut().drain(..).collect();
        for f in new {
            tasks.insert(next_id, f);
            queue.lock().unwrap().push_back(next_id);
            next_id += 1;
        }
        let next = queue.lock().unwrap().pop_front();
        match next {
            Some(id) => {
                let waker = Waker::from(Arc::new(TaskWaker { id, queue: queue.clone() }));
                let mut cx = Context::from_waker(&waker);
                if let Some(t) = tasks.get_mut(&id) {
                    polls += 1;
                    if t.as_mut().poll(&mut cx).is_ready() {
                        tasks.remove(&id);
                    }
                }
            }
            None => {
                if !shared.spawned.borrow().is_empty() {
                    continue;
                }
                // advance the virtual clock to the next timer
                let next = shared.timers.borrow_mut().pop();
                match next {
                    Some(Reverse((deadline, _, slot))) => {
                        shared.now.set(shared.now.get().max(deadline));
                        if let Some(w) = shared.timer_wakers.borrow_mut().remove(&slot) {
                            w.wake();
                        }
                    }
                    None => break,
                }
            }
        }
    }
    let mut out = shared.log.borrow().clone();
    out.push(format!("done: {} polls, {} tasks left, clock {}", polls, tasks.len(), shared.now.get()));
    RT.with(|r| *r.borrow_mut() = None);
    out
}

// ---------------- event bus ----------------
type Handler = Box<dyn Fn(&dyn Any)>;

#[derive(Default)]
struct Bus {
    handlers: RefCell<HashMap<TypeId, Vec<(String, Handler)>>>,
}

impl Bus {
    fn subscribe<E: 'static>(&self, name: &str, f: impl Fn(&E) + 'static) {
        self.handlers.borrow_mut().entry(TypeId::of::<E>()).or_default().push((name.to_string(), Box::new(move |e: &dyn Any| f(e.downcast_ref::<E>().unwrap()))));
    }
    fn publish<E: 'static>(&self, e: E) -> usize {
        let hs = self.handlers.borrow();
        match hs.get(&TypeId::of::<E>()) {
            Some(list) => {
                for (_, h) in list {
                    h(&e);
                }
                list.len()
            }
            None => 0,
        }
    }
}

#[derive(Debug, Clone)]
struct OrderPlaced {
    id: u32,
    qty: u32,
}
#[derive(Debug)]
struct Shutdown(&'static str);

async fn worker(name: &'static str, rx: Rc<Receiver<(u32, u64)>>, sem: Semaphore, done: Sender<String>) {
    loop {
        let Some((job, cost)) = rx.recv().await else { break };
        let permit = sem.acquire().await;
        log(format!("{} starts job {} (cost {})", name, job, cost));
        sleep(cost).await;
        drop(permit);
        log(format!("{} finished job {}", name, job));
        let _ = done.send(format!("{}:{}", name, job)).await;
    }
    log(format!("{} exits", name));
}

async fn app() {
    let (tx, rx) = channel::<(u32, u64)>(2);
    let rx = Rc::new(rx);
    let (dtx, drx) = channel::<String>(16);
    let sem = Semaphore::new(2);
    for name in ["w1", "w2", "w3"] {
        spawn(worker(name, rx.clone(), sem.clone(), dtx.clone()));
    }
    drop(dtx);
    let producer = async move {
        for (i, cost) in [5u64, 3, 8, 2, 7, 1].iter().enumerate() {
            tx.send((i as u32, *cost)).await.ok();
            log(format!("queued job {}", i));
        }
        log("producer done".to_string());
    };
    let collector = async move {
        let mut got = vec![];
        while let Some(s) = drx.recv().await {
            got.push(s);
        }
        got
    };
    let ((), results) = join2(producer, collector).await;
    log(format!("results {:?}", results));
    match select(sleep(3), sleep(10)).await {
        Either::Left(()) => log("short timer won".into()),
        Either::Right(()) => log("long timer won".into()),
    }
    let (a, b) = join2(async { sleep(4).await; 40 }, async { sleep(2).await; 2 }).await;
    log(format!("joined {} + {} = {}", a, b, a + b));

    let bus = Rc::new(Bus::default());
    let total = Rc::new(Cell::new(0u32));
    let t2 = total.clone();
    bus.subscribe::<OrderPlaced>("tally", move |o| t2.set(t2.get() + o.qty));
    bus.subscribe::<OrderPlaced>("ship", |o: &OrderPlaced| {
        let o = o.clone();
        spawn(async move {
            sleep(o.qty as u64).await;
            log(format!("shipped order {} ({} units)", o.id, o.qty));
        })
    });
    bus.subscribe::<Shutdown>("audit", |s| log(format!("shutdown requested: {}", s.0)));
    for (id, qty) in [(1, 3), (2, 1), (3, 2)] {
        let n = bus.publish(OrderPlaced { id, qty });
        log(format!("order {} delivered to {} handlers", id, n));
    }
    log(format!("unhandled event delivered to {} handlers", bus.publish(42u8)));
    sleep(5).await;
    bus.publish(Shutdown("end of day"));
    log(format!("total units {}", total.get()));
}

fn main() {
    for line in run(app()) {
        println!("{}", line);
    }
    // a second, independent run proves the runtime resets cleanly
    let out = run(async {
        let (tx, rx) = channel::<u32>(1);
        spawn(async move {
            for i in 0..3 {
                tx.send(i * i).await.ok();
                sleep(1).await;
            }
        });
        let mut sum = 0;
        while let Some(v) = rx.recv().await {
            sum += v;
            log(format!("got {}", v));
        }
        log(format!("sum {}", sum));
    });
    println!("{}", out.join("\n"));
}
