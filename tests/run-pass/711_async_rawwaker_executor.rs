// A single-threaded async executor built on RawWaker with a vtable, task
// queue in Rc<RefCell<VecDeque>>, a virtual-time timer reactor, join of
// heterogeneous futures and a oneshot channel.
use std::cell::RefCell;
use std::collections::{BTreeMap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

thread_local! {
    static READY: RefCell<VecDeque<usize>> = RefCell::new(VecDeque::new());
    static NOW: RefCell<u64> = RefCell::new(0);
    static TIMERS: RefCell<BTreeMap<(u64, usize), Waker>> = RefCell::new(BTreeMap::new());
    static WAKES: RefCell<usize> = RefCell::new(0);
}

fn vt_clone(p: *const ()) -> RawWaker { RawWaker::new(p, &VTABLE) }
fn vt_wake(p: *const ()) { vt_wake_by_ref(p) }
fn vt_wake_by_ref(p: *const ()) {
    WAKES.with(|w| *w.borrow_mut() += 1);
    READY.with(|q| {
        let id = p as usize;
        let mut q = q.borrow_mut();
        if !q.contains(&id) { q.push_back(id); }
    });
}
fn vt_drop(_: *const ()) {}
static VTABLE: RawWakerVTable = RawWakerVTable::new(vt_clone, vt_wake, vt_wake_by_ref, vt_drop);

fn waker_for(id: usize) -> Waker {
    unsafe { Waker::from_raw(RawWaker::new(id as *const (), &VTABLE)) }
}

struct Sleep { until: u64, seq: usize }
impl Future for Sleep {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        let now = NOW.with(|n| *n.borrow());
        if now >= self.until {
            Poll::Ready(now)
        } else {
            TIMERS.with(|t| t.borrow_mut().insert((self.until, self.seq), cx.waker().clone()));
            Poll::Pending
        }
    }
}
static mut SEQ: usize = 0;
fn sleep(d: u64) -> Sleep {
    let seq = unsafe { SEQ += 1; SEQ };
    Sleep { until: NOW.with(|n| *n.borrow()) + d, seq }
}

struct Shared<T> { val: Option<T>, waker: Option<Waker> }
struct Sender<T>(Rc<RefCell<Shared<T>>>);
struct Receiver<T>(Rc<RefCell<Shared<T>>>);
fn oneshot<T>() -> (Sender<T>, Receiver<T>) {
    let s = Rc::new(RefCell::new(Shared { val: None, waker: None }));
    (Sender(s.clone()), Receiver(s))
}
impl<T> Sender<T> {
    fn send(self, v: T) {
        let mut s = self.0.borrow_mut();
        s.val = Some(v);
        if let Some(w) = s.waker.take() { w.wake(); }
    }
}
impl<T> Future for Receiver<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut s = self.0.borrow_mut();
        match s.val.take() {
            Some(v) => Poll::Ready(v),
            None => { s.waker = Some(cx.waker().clone()); Poll::Pending }
        }
    }
}

struct Join2<A: Future, B: Future> { a: Pin<Box<A>>, b: Pin<Box<B>>, ra: Option<A::Output>, rb: Option<B::Output> }
impl<A: Future, B: Future> Future for Join2<A, B> where A::Output: Unpin, B::Output: Unpin {
    type Output = (A::Output, B::Output);
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.ra.is_none() { if let Poll::Ready(v) = this.a.as_mut().poll(cx) { this.ra = Some(v); } }
        if this.rb.is_none() { if let Poll::Ready(v) = this.b.as_mut().poll(cx) { this.rb = Some(v); } }
        if this.ra.is_some() && this.rb.is_some() {
            Poll::Ready((this.ra.take().unwrap(), this.rb.take().unwrap()))
        } else { Poll::Pending }
    }
}
fn join2<A: Future, B: Future>(a: A, b: B) -> Join2<A, B> { Join2 { a: Box::pin(a), b: Box::pin(b), ra: None, rb: None } }

type Log = Rc<RefCell<Vec<String>>>;

fn log(l: &Log, s: String) {
    let t = NOW.with(|n| *n.borrow());
    l.borrow_mut().push(format!("[t={:>3}] {}", t, s));
}

async fn worker(name: &'static str, steps: u32, delay: u64, l: Log) -> u32 {
    let mut acc = 0;
    for i in 0..steps {
        sleep(delay).await;
        acc += i * delay as u32;
        log(&l, format!("{} step {}", name, i));
    }
    acc
}

fn run(tasks: Vec<Pin<Box<dyn Future<Output = ()>>>>) -> usize {
    let mut tasks: Vec<Option<Pin<Box<dyn Future<Output = ()>>>>> = tasks.into_iter().map(Some).collect();
    READY.with(|q| q.borrow_mut().extend(0..tasks.len()));
    let mut polls = 0;
    loop {
        while let Some(id) = READY.with(|q| q.borrow_mut().pop_front()) {
            let Some(fut) = tasks[id].as_mut() else { continue };
            polls += 1;
            let w = waker_for(id);
            let mut cx = Context::from_waker(&w);
            if fut.as_mut().poll(&mut cx).is_ready() { tasks[id] = None; }
        }
        let next = TIMERS.with(|t| t.borrow_mut().pop_first());
        match next {
            Some(((when, _), w)) => { NOW.with(|n| *n.borrow_mut() = when); w.wake(); }
            None => break,
        }
    }
    assert!(tasks.iter().all(Option::is_none), "deadlock");
    polls
}

fn main() {
    let l: Log = Rc::new(RefCell::new(Vec::new()));
    let (tx, rx) = oneshot::<String>();
    let (tx2, rx2) = oneshot::<u32>();
    let l1 = l.clone();
    let l2 = l.clone();
    let l3 = l.clone();
    let tasks: Vec<Pin<Box<dyn Future<Output = ()>>>> = vec![
        Box::pin(async move {
            let (a, b) = join2(worker("alpha", 3, 10, l1.clone()), worker("beta", 2, 25, l1.clone())).await;
            log(&l1, format!("joined alpha={} beta={}", a, b));
            tx.send(format!("sum={}", a + b));
        }),
        Box::pin(async move {
            let msg = rx.await;
            log(&l2, format!("got message {}", msg));
            let t = sleep(7).await;
            tx2.send(t as u32 * 2);
        }),
        Box::pin(async move {
            log(&l3, "waiting for number".to_string());
            let n = rx2.await;
            log(&l3, format!("number {}", n));
            let mut v = Vec::new();
            for d in [5u64, 1, 3] {
                v.push(sleep(d).await);
            }
            log(&l3, format!("timestamps {:?}", v));
        }),
    ];
    let polls = run(tasks);
    for line in l.borrow().iter() { println!("{}", line); }
    println!("polls={} wakes={} final time={}", polls, WAKES.with(|w| *w.borrow()), NOW.with(|n| *n.borrow()));
    let w = waker_for(99);
    let w2 = w.clone();
    w2.wake();
    w.wake_by_ref();
    println!("ready queue: {:?}", READY.with(|q| q.borrow().iter().copied().collect::<Vec<_>>()));
}
