use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker, Wake};

// A future that yields N times before completing
struct YieldN {
    left: u32,
}

impl Future for YieldN {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        if self.left == 0 {
            Poll::Ready(7)
        } else {
            self.left -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

async fn add_later(a: u32, b: u32) -> u32 {
    let extra = YieldN { left: 2 }.await;
    a + b + extra
}

async fn chain(log: Rc<RefCell<Vec<String>>>, name: &'static str, n: u32) -> u32 {
    let mut total = 0;
    for i in 0..n {
        log.borrow_mut().push(format!("{}:{}", name, i));
        total += add_later(i, 1).await;
    }
    total
}

struct NoopWake;
impl Wake for NoopWake {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(fut: F) -> (F::Output, u32) {
    let waker = Waker::from(Arc::new(NoopWake));
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

// round-robin executor with boxed futures
struct Exec {
    tasks: VecDeque<(usize, Pin<Box<dyn Future<Output = u32>>>)>,
}

fn raw_waker() -> RawWaker {
    fn clone(_: *const ()) -> RawWaker {
        raw_waker()
    }
    fn noop(_: *const ()) {}
    static VT: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    RawWaker::new(std::ptr::null(), &VT)
}

impl Exec {
    fn run(&mut self) -> Vec<(usize, u32)> {
        let waker = unsafe { Waker::from_raw(raw_waker()) };
        let mut cx = Context::from_waker(&waker);
        let mut done = vec![];
        while let Some((id, mut f)) = self.tasks.pop_front() {
            match f.as_mut().poll(&mut cx) {
                Poll::Ready(v) => done.push((id, v)),
                Poll::Pending => self.tasks.push_back((id, f)),
            }
        }
        done
    }
}

trait AsyncSource {
    fn fetch(&self, k: u32) -> Pin<Box<dyn Future<Output = String> + '_>>;
}

struct Db {
    prefix: String,
}
impl AsyncSource for Db {
    fn fetch(&self, k: u32) -> Pin<Box<dyn Future<Output = String> + '_>> {
        Box::pin(async move {
            YieldN { left: k }.await;
            format!("{}{}", self.prefix, k)
        })
    }
}

async fn use_source(s: &dyn AsyncSource) -> Vec<String> {
    let mut out = vec![];
    for k in 1..=3 {
        out.push(s.fetch(k).await);
    }
    out
}

fn main() {
    let (v, polls) = block_on(add_later(2, 3));
    println!("{} after {} polls", v, polls);

    let log = Rc::new(RefCell::new(vec![]));
    let mut ex = Exec { tasks: VecDeque::new() };
    ex.tasks.push_back((1, Box::pin(chain(log.clone(), "a", 2))));
    ex.tasks.push_back((2, Box::pin(chain(log.clone(), "b", 3))));
    ex.tasks.push_back((3, Box::pin(async { 99 })));
    let captured = 5;
    ex.tasks.push_back((4, Box::pin(async move { captured * 2 })));
    let done = ex.run();
    println!("{:?}", done);
    println!("{:?}", log.borrow());

    let db = Db { prefix: "row".into() };
    let (rows, polls) = block_on(use_source(&db));
    println!("{:?} {}", rows, polls);

    let closure_fut = |x: u32| async move { x + 1 };
    println!("{:?}", block_on(closure_fut(41)).0);
    let nested = async {
        let a = async { 1 }.await;
        let b = async { a + 1 }.await;
        let r: Result<u32, String> = async { Ok::<u32, String>(b * 10) }.await;
        r
    };
    println!("{:?}", block_on(nested).0);

    async fn try_stuff(fail: bool) -> Result<u32, String> {
        let x: u32 = async { if fail { Err("bad".to_string()) } else { Ok(3) } }.await?;
        Ok(x * 2)
    }
    println!("{:?} {:?}", block_on(try_stuff(false)).0, block_on(try_stuff(true)).0);
    println!("{:?}", std::future::ready(5).into_poll_check());
}

trait PollCheck {
    fn into_poll_check(self) -> u32;
}
impl PollCheck for std::future::Ready<u32> {
    fn into_poll_check(self) -> u32 {
        block_on(self).0
    }
}
