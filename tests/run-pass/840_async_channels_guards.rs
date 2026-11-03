use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct NoopWake;
impl Wake for NoopWake { fn wake(self: Arc<Self>) {} }

type Task = Pin<Box<dyn Future<Output = ()>>>;
struct Exec { q: RefCell<VecDeque<(usize, Task)>>, log: RefCell<Vec<String>> }

struct Yield { n: u32 }
impl Future for Yield {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        if self.n == 0 { Poll::Ready(7) } else { self.n -= 1; cx.waker().wake_by_ref(); Poll::Pending }
    }
}

struct Chan<T> { buf: RefCell<VecDeque<T>>, closed: RefCell<bool> }
struct Recv<'a, T> { ch: &'a Chan<T> }
impl<'a, T> Future for Recv<'a, T> {
    type Output = Option<T>;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<Option<T>> {
        if let Some(v) = self.ch.buf.borrow_mut().pop_front() { Poll::Ready(Some(v)) }
        else if *self.ch.closed.borrow() { Poll::Ready(None) } else { Poll::Pending }
    }
}

async fn producer(id: usize, ch: Rc<Chan<String>>, n: u32, log: Rc<RefCell<Vec<String>>>) {
    for i in 0..n {
        ch.buf.borrow_mut().push_back(format!("p{}-{}", id, i));
        log.borrow_mut().push(format!("produce p{}-{}", id, i));
        Yield { n: id as u32 }.await;
    }
}

async fn consumer(ch: Rc<Chan<String>>, log: Rc<RefCell<Vec<String>>>) -> usize {
    let mut got = 0;
    while let Some(m) = (Recv { ch: &ch }).await {
        log.borrow_mut().push(format!("consume {}", m));
        got += 1;
    }
    got
}

fn fib(n: u32) -> Pin<Box<dyn Future<Output = u64>>> {
    Box::pin(async move { if n < 2 { n as u64 } else { let a = fib(n - 1).await; let b = fib(n - 2).await; a + b } })
}

struct Guard(String, Rc<RefCell<Vec<String>>>);
impl Drop for Guard { fn drop(&mut self) { self.1.borrow_mut().push(format!("drop {}", self.0)); } }

fn block_on<F: Future>(f: F) -> F::Output {
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    let mut polls = 0;
    loop { polls += 1; if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { println!("(block_on polls={})", polls); return v; } }
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let ch = Rc::new(Chan { buf: RefCell::new(VecDeque::new()), closed: RefCell::new(false) });
    let ex = Exec { q: RefCell::new(VecDeque::new()), log: RefCell::new(vec![]) };
    ex.q.borrow_mut().push_back((0, Box::pin(producer(1, ch.clone(), 3, log.clone()))));
    ex.q.borrow_mut().push_back((1, Box::pin(producer(2, ch.clone(), 2, log.clone()))));
    let result = Rc::new(RefCell::new(0));
    let (r2, ch2, l2) = (result.clone(), ch.clone(), log.clone());
    ex.q.borrow_mut().push_back((2, Box::pin(async move { let n = consumer(ch2, l2).await; *r2.borrow_mut() = n; })));
    let waker = Waker::from(Arc::new(NoopWake));
    let mut cx = Context::from_waker(&waker);
    let mut rounds = 0;
    loop {
        let next = ex.q.borrow_mut().pop_front();
        let Some((id, mut t)) = next else { break };
        rounds += 1;
        match t.as_mut().poll(&mut cx) {
            Poll::Ready(()) => { ex.log.borrow_mut().push(format!("task {} done at round {}", id, rounds)); if id < 2 && ex.q.borrow().iter().filter(|(i, _)| *i < 2).count() == 0 { *ch.closed.borrow_mut() = true; } }
            Poll::Pending => ex.q.borrow_mut().push_back((id, t)),
        }
        if rounds > 100 { break; }
    }
    for l in log.borrow().iter() { println!("{}", l); }
    for l in ex.log.borrow().iter() { println!("{}", l); }
    println!("consumed {}", result.borrow());
    println!("fib {}", block_on(fib(15)));
    let glog = Rc::new(RefCell::new(Vec::new()));
    let gl = glog.clone();
    let v = block_on(async move {
        let _g1 = Guard("outer".into(), gl.clone());
        let x = Yield { n: 2 }.await;
        let inner = async { let _g2 = Guard("inner".into(), gl.clone()); Yield { n: 1 }.await + 1 };
        let y = inner.await;
        let data = vec![1, 2, 3];
        let r = &data;
        let z = async { r.iter().sum::<i32>() }.await;
        x + y + z as u32
    });
    println!("v={} log={:?}", v, glog.borrow());
    let s = String::from("borrowed");
    let fut = async { s.len() };
    println!("{}", block_on(fut));
    let closure = |k: u32| async move { k * 3 };
    println!("{}", block_on(async { closure(4).await + closure(5).await }));
    let futs: Vec<Pin<Box<dyn Future<Output = u32>>>> = (0..4).map(|i| Box::pin(async move { Yield { n: i }.await + i }) as Pin<Box<dyn Future<Output = u32>>>).collect();
    let total = block_on(async { let mut t = 0; for f in futs { t += f.await; } t });
    println!("total {}", total);
    let ready = block_on(std::future::ready(42));
    println!("{}", ready);
    let mut pin_val = std::pin::pin!(async { 9 });
    println!("{:?}", pin_val.as_mut().poll(&mut cx));
}
