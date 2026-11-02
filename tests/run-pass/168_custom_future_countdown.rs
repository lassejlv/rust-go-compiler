use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

struct Countdown {
    name: &'static str,
    remaining: u32,
    log: Rc<RefCell<Vec<String>>>,
}

impl Future for Countdown {
    type Output = String;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<String> {
        if self.remaining == 0 {
            self.log.borrow_mut().push(format!("{} done", self.name));
            Poll::Ready(format!("{} finished", self.name))
        } else {
            let msg = format!("{} pending {}", self.name, self.remaining);
            self.log.borrow_mut().push(msg);
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

struct FlagWaker {
    woken: Mutex<u32>,
}
impl Wake for FlagWaker {
    fn wake(self: Arc<Self>) {
        *self.woken.lock().unwrap() += 1;
    }
}

fn block_on<F: Future>(f: F) -> F::Output {
    let fw = Arc::new(FlagWaker { woken: Mutex::new(0) });
    let waker = Waker::from(fw.clone());
    let mut cx = Context::from_waker(&waker);
    let mut f = Box::pin(f);
    loop {
        match f.as_mut().poll(&mut cx) {
            Poll::Ready(v) => {
                println!("wake count {}", fw.woken.lock().unwrap());
                return v;
            }
            Poll::Pending => {
                // only re-poll if woken
                assert!(*fw.woken.lock().unwrap() > 0);
            }
        }
    }
}

struct Pair<A, B> {
    a: Option<A>,
    b: Option<B>,
}

// A Future combinator over two Unpin futures.
struct Join2<FA: Future, FB: Future> {
    fa: Pin<Box<FA>>,
    fb: Pin<Box<FB>>,
    out: Pair<FA::Output, FB::Output>,
}

impl<FA: Future, FB: Future> Future for Join2<FA, FB>
where
    FA::Output: Unpin,
    FB::Output: Unpin,
{
    type Output = (FA::Output, FB::Output);
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = &mut *self;
        if this.out.a.is_none() {
            if let Poll::Ready(v) = this.fa.as_mut().poll(cx) {
                this.out.a = Some(v);
            }
        }
        if this.out.b.is_none() {
            if let Poll::Ready(v) = this.fb.as_mut().poll(cx) {
                this.out.b = Some(v);
            }
        }
        if this.out.a.is_some() && this.out.b.is_some() {
            Poll::Ready((this.out.a.take().unwrap(), this.out.b.take().unwrap()))
        } else {
            Poll::Pending
        }
    }
}

fn join2<FA: Future, FB: Future>(fa: FA, fb: FB) -> Join2<FA, FB> {
    Join2 { fa: Box::pin(fa), fb: Box::pin(fb), out: Pair { a: None, b: None } }
}

fn main() {
    let log = Rc::new(RefCell::new(Vec::new()));
    let r = block_on(Countdown { name: "solo", remaining: 3, log: log.clone() });
    println!("{}", r);
    println!("{:?}", log.borrow());
    log.borrow_mut().clear();

    let (x, y) = block_on(join2(
        Countdown { name: "A", remaining: 2, log: log.clone() },
        Countdown { name: "B", remaining: 4, log: log.clone() },
    ));
    println!("{} / {}", x, y);
    for line in log.borrow().iter() {
        println!("  {}", line);
    }
    log.borrow_mut().clear();

    let l2 = log.clone();
    let total = block_on(async move {
        let mut q: VecDeque<u32> = (1..=3).collect();
        let mut sum = 0;
        while let Some(n) = q.pop_front() {
            let s = Countdown { name: "loop", remaining: n, log: l2.clone() }.await;
            sum += n * 10 + s.len() as u32;
        }
        sum
    });
    println!("total {}", total);
    println!("log entries {}", log.borrow().len());
}
