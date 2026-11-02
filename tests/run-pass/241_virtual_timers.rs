// A deterministic executor with a virtual clock and timer heap: timers register their wakers,
// the executor jumps time to the next deadline when no task is runnable (like a test dispatcher).
use std::cell::RefCell;
use std::cmp::Reverse;
use std::collections::{BinaryHeap, VecDeque};
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::{Arc, Mutex};
use std::task::{Context, Poll, Wake, Waker};

#[derive(Default)]
struct State {
    now: u64,
    timers: BinaryHeap<Reverse<(u64, u64)>>, // (deadline, seq)
    timer_wakers: Vec<(u64, Waker)>,          // (seq, waker)
    seq: u64,
    log: Vec<String>,
}

#[derive(Clone, Default)]
struct Dispatcher(Rc<RefCell<State>>);

struct Timer {
    d: Dispatcher,
    deadline: u64,
    registered: bool,
}

impl Future for Timer {
    type Output = u64;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        let mut s = self.d.0.borrow_mut();
        if s.now >= self.deadline {
            return Poll::Ready(s.now);
        }
        if !self.registered {
            let seq = s.seq;
            s.seq += 1;
            s.timers.push(Reverse((self.deadline, seq)));
            s.timer_wakers.push((seq, cx.waker().clone()));
            drop(s);
            self.registered = true;
        }
        Poll::Pending
    }
}

impl Dispatcher {
    fn timer(&self, after: u64) -> Timer {
        let deadline = self.0.borrow().now + after;
        Timer { d: self.clone(), deadline, registered: false }
    }
    fn now(&self) -> u64 {
        self.0.borrow().now
    }
    fn log(&self, msg: String) {
        let now = self.now();
        self.0.borrow_mut().log.push(format!("[t={:>3}] {}", now, msg));
    }
}

struct TaskWaker {
    id: usize,
    ready: Arc<Mutex<VecDeque<usize>>>,
}
impl Wake for TaskWaker {
    fn wake(self: Arc<Self>) {
        self.ready.lock().unwrap().push_back(self.id);
    }
}

fn run(d: &Dispatcher, futs: Vec<Pin<Box<dyn Future<Output = ()>>>>) -> (usize, usize) {
    let ready = Arc::new(Mutex::new((0..futs.len()).collect::<VecDeque<_>>()));
    let mut tasks: Vec<Option<Pin<Box<dyn Future<Output = ()>>>>> = futs.into_iter().map(Some).collect();
    let mut polls = 0;
    let mut jumps = 0;
    loop {
        let next = ready.lock().unwrap().pop_front();
        if let Some(id) = next {
            if let Some(t) = tasks[id].as_mut() {
                let w = Waker::from(Arc::new(TaskWaker { id, ready: ready.clone() }));
                polls += 1;
                if t.as_mut().poll(&mut Context::from_waker(&w)).is_ready() {
                    tasks[id] = None;
                }
            }
            continue;
        }
        // No runnable task: advance the clock to the earliest timer and fire all due timers.
        let mut s = d.0.borrow_mut();
        let Some(Reverse((deadline, _))) = s.timers.peek().copied() else { break };
        s.now = deadline;
        jumps += 1;
        let mut fire = vec![];
        while let Some(Reverse((dl, seq))) = s.timers.peek().copied() {
            if dl > s.now {
                break;
            }
            s.timers.pop();
            fire.push(seq);
        }
        let mut wakers = vec![];
        s.timer_wakers.retain(|(seq, w)| {
            if fire.contains(seq) {
                wakers.push(w.clone());
                false
            } else {
                true
            }
        });
        drop(s);
        for w in wakers {
            w.wake();
        }
    }
    (polls, jumps)
}

async fn heartbeat(d: Dispatcher, name: &'static str, period: u64, count: u32) {
    for i in 1..=count {
        d.timer(period).await;
        d.log(format!("{} beat {}", name, i));
    }
}

async fn debounce(d: Dispatcher, events: Vec<u64>, quiet: u64) {
    // events are absolute times; emit only after `quiet` ticks without a new event
    let mut last_emit = None;
    let mut i = 0;
    while i < events.len() {
        let t = events[i];
        if d.now() < t {
            d.timer(t - d.now()).await;
        }
        let next = events.get(i + 1).copied();
        match next {
            Some(n) if n - t < quiet => {
                d.log(format!("debounce: event@{} suppressed", t));
            }
            _ => {
                d.timer(quiet).await;
                d.log(format!("debounce: emit event@{}", t));
                last_emit = Some(t);
            }
        }
        i += 1;
    }
    d.log(format!("debounce done, last={:?}", last_emit));
}

fn main() {
    let d = Dispatcher::default();
    let tasks: Vec<Pin<Box<dyn Future<Output = ()>>>> = vec![
        Box::pin(heartbeat(d.clone(), "fast", 3, 4)),
        Box::pin(heartbeat(d.clone(), "slow", 5, 3)),
        Box::pin(debounce(d.clone(), vec![1, 2, 4, 10, 30, 31], 4)),
        Box::pin({
            let d = d.clone();
            async move {
                let t = d.timer(0).await;
                d.log(format!("zero timer ready immediately at {}", t));
                let t = d.timer(100).await;
                d.log(format!("long timer at {}", t));
            }
        }),
    ];
    let (polls, jumps) = run(&d, tasks);
    for l in d.0.borrow().log.iter() {
        println!("{}", l);
    }
    println!("polls={} jumps={} end={}", polls, jumps, d.now());
}
