// Hand-written executor using std::task::Wake with Arc, a task queue,
// and futures that yield a number of times before completing.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

struct TaskWaker {
    id: usize,
    queue: Arc<Mutex<VecDeque<usize>>>,
    wakes: AtomicUsize,
}

impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref();
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.wakes.fetch_add(1, Ordering::SeqCst);
        self.queue.lock().unwrap().push_back(self.id);
    }
}

struct YieldN {
    remaining: u32,
    name: &'static str,
}

impl Future for YieldN {
    type Output = &'static str;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        if self.remaining == 0 {
            Poll::Ready(self.name)
        } else {
            self.remaining -= 1;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

async fn worker(name: &'static str, n: u32, log: Rc<RefCell<Vec<String>>>) -> usize {
    log.borrow_mut().push(format!("{} start", name));
    let r = YieldN { remaining: n, name }.await;
    log.borrow_mut().push(format!("{} mid {}", name, r));
    let r2 = YieldN { remaining: 1, name: "inner" }.await;
    log.borrow_mut().push(format!("{} end {}", name, r2));
    name.len() * 10 + n as usize
}

type Task = Pin<Box<dyn Future<Output = usize>>>;

fn main() {
    let queue = Arc::new(Mutex::new(VecDeque::new()));
    let log = Rc::new(RefCell::new(Vec::new()));
    let mut tasks: Vec<Option<Task>> = Vec::new();
    let mut wakers = Vec::new();
    let specs = [("alpha", 2u32), ("be", 0), ("gamma", 3)];
    for (i, (name, n)) in specs.iter().enumerate() {
        tasks.push(Some(Box::pin(worker(name, *n, log.clone()))));
        let w = Arc::new(TaskWaker { id: i, queue: queue.clone(), wakes: AtomicUsize::new(0) });
        wakers.push(w);
        queue.lock().unwrap().push_back(i);
    }
    let mut results = vec![0usize; tasks.len()];
    let mut polls = 0;
    loop {
        let next = queue.lock().unwrap().pop_front();
        let Some(id) = next else { break };
        let Some(fut) = tasks[id].as_mut() else { continue };
        let waker = Waker::from(wakers[id].clone());
        let mut cx = Context::from_waker(&waker);
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            results[id] = v;
            tasks[id] = None;
            log.borrow_mut().push(format!("task {} done -> {}", id, v));
        }
    }
    for l in log.borrow().iter() {
        println!("{}", l);
    }
    println!("results {:?} polls {}", results, polls);
    for (i, w) in wakers.iter().enumerate() {
        println!("waker {} wakes={} strong={}", i, w.wakes.load(Ordering::SeqCst), Arc::strong_count(w));
    }
}
