// An async-task-like single-threaded executor: Arc<Task> + std::task::Wake, a run queue,
// JoinHandles with shared result slots, and futures that yield.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

type LocalBoxFuture = Pin<Box<dyn Future<Output = ()> + Send>>;

struct Task {
    id: usize,
    future: Mutex<Option<LocalBoxFuture>>,
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    wakes: Mutex<u32>,
}

impl Wake for Task {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        *self.wakes.lock().unwrap() += 1;
        self.queue.lock().unwrap().push_back(self.clone());
    }
}

struct Executor {
    queue: Arc<Mutex<VecDeque<Arc<Task>>>>,
    next_id: usize,
    log: Vec<String>,
}

struct JoinHandle<T> {
    slot: Arc<Mutex<(Option<T>, Option<Waker>)>>,
}

impl<T> Future for JoinHandle<T> {
    type Output = T;
    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<T> {
        let mut s = self.slot.lock().unwrap();
        match s.0.take() {
            Some(v) => Poll::Ready(v),
            None => {
                s.1 = Some(cx.waker().clone());
                Poll::Pending
            }
        }
    }
}

impl Executor {
    fn new() -> Self {
        Executor { queue: Arc::new(Mutex::new(VecDeque::new())), next_id: 0, log: vec![] }
    }

    fn spawn<F, T>(&mut self, fut: F) -> JoinHandle<T>
    where
        F: Future<Output = T> + Send + 'static,
        T: Send + 'static,
    {
        let slot = Arc::new(Mutex::new((None, None::<Waker>)));
        let slot2 = slot.clone();
        let wrapped = async move {
            let v = fut.await;
            let mut s = slot2.lock().unwrap();
            s.0 = Some(v);
            if let Some(w) = s.1.take() {
                w.wake();
            }
        };
        let task = Arc::new(Task {
            id: self.next_id,
            future: Mutex::new(Some(Box::pin(wrapped))),
            queue: self.queue.clone(),
            wakes: Mutex::new(0),
        });
        self.next_id += 1;
        self.queue.lock().unwrap().push_back(task);
        JoinHandle { slot }
    }

    fn run(&mut self) -> usize {
        let mut polls = 0;
        loop {
            let next = self.queue.lock().unwrap().pop_front();
            let Some(task) = next else { break };
            let mut slot = task.future.lock().unwrap();
            if let Some(mut fut) = slot.take() {
                let waker = Waker::from(task.clone());
                let mut cx = Context::from_waker(&waker);
                polls += 1;
                match fut.as_mut().poll(&mut cx) {
                    Poll::Ready(()) => self.log.push(format!("task {} done (woken {}x)", task.id, task.wakes.lock().unwrap())),
                    Poll::Pending => *slot = Some(fut),
                }
            }
        }
        polls
    }
}

// A future that yields `n` times before completing
struct YieldN {
    left: u32,
}
impl Future for YieldN {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.left == 0 {
            Poll::Ready(())
        } else {
            self.left -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

fn yield_now() -> YieldN {
    YieldN { left: 1 }
}

fn main() {
    let trace = Arc::new(Mutex::new(Vec::<String>::new()));
    let mut ex = Executor::new();

    let mut handles = vec![];
    for i in 0..3u32 {
        let t = trace.clone();
        handles.push(ex.spawn(async move {
            let mut acc = 0u64;
            for step in 0..=i {
                t.lock().unwrap().push(format!("w{} step{}", i, step));
                acc += (step as u64 + 1) * 10;
                yield_now().await;
            }
            acc
        }));
    }

    let t = trace.clone();
    let summary = ex.spawn(async move {
        let mut total = 0;
        for (i, h) in handles.into_iter().enumerate() {
            let v = h.await;
            t.lock().unwrap().push(format!("joined w{} = {}", i, v));
            total += v;
        }
        YieldN { left: 3 }.await;
        format!("total={}", total)
    });

    let t = trace.clone();
    let printer = ex.spawn(async move {
        let s = summary.await;
        t.lock().unwrap().push(format!("summary says {}", s));
        s.len()
    });

    let polls = ex.run();
    for line in trace.lock().unwrap().iter() {
        println!("{}", line);
    }
    for l in &ex.log {
        println!("{}", l);
    }
    println!("polls={}", polls);

    // The final handle is ready; poll it manually with a noop-style waker.
    let flag = Rc::new(RefCell::new(0));
    struct CountWake(Mutex<u32>);
    impl Wake for CountWake {
        fn wake(self: Arc<Self>) {
            *self.0.lock().unwrap() += 1;
        }
    }
    let cw = Arc::new(CountWake(Mutex::new(0)));
    let waker = Waker::from(cw.clone());
    let mut cx = Context::from_waker(&waker);
    let mut p = printer;
    match Pin::new(&mut p).poll(&mut cx) {
        Poll::Ready(n) => *flag.borrow_mut() = n,
        Poll::Pending => println!("unexpected pending"),
    }
    println!("printer result={} wakes={}", flag.borrow(), cw.0.lock().unwrap());
}
