// block_on that parks the thread, a waker that unparks it, and a timer thread waking
// futures from another thread; shared state through Arc<Mutex<..>>.
use std::collections::BinaryHeap;
use std::cmp::Reverse;
use std::future::Future;
use std::pin::Pin;
use std::sync::atomic::{AtomicBool, AtomicUsize, Ordering};
use std::sync::{Arc, Condvar, Mutex};
use std::task::{Context, Poll, Wake, Waker};
use std::thread::{self, Thread};
use std::time::Duration;

struct ThreadWaker {
    thread: Thread,
    woken: AtomicBool,
    count: AtomicUsize,
}
impl Wake for ThreadWaker {
    fn wake(self: Arc<Self>) {
        self.wake_by_ref()
    }
    fn wake_by_ref(self: &Arc<Self>) {
        self.count.fetch_add(1, Ordering::SeqCst);
        self.woken.store(true, Ordering::Release);
        self.thread.unpark();
    }
}

fn block_on<F: Future>(fut: F) -> (F::Output, usize) {
    let tw = Arc::new(ThreadWaker { thread: thread::current(), woken: AtomicBool::new(false), count: AtomicUsize::new(0) });
    let waker = Waker::from(tw.clone());
    let mut cx = Context::from_waker(&waker);
    let mut fut = std::pin::pin!(fut);
    loop {
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            return (v, tw.count.load(Ordering::SeqCst));
        }
        while !tw.woken.swap(false, Ordering::Acquire) {
            thread::park();
        }
    }
}

struct TimerShared {
    queue: Mutex<(BinaryHeap<Reverse<(u64, usize)>>, Vec<Option<Waker>>, Vec<bool>, bool)>,
    cv: Condvar,
}

#[derive(Clone)]
struct Timer(Arc<TimerShared>);

impl Timer {
    fn start() -> (Timer, thread::JoinHandle<u32>) {
        let shared = Arc::new(TimerShared { queue: Mutex::new((BinaryHeap::new(), Vec::new(), Vec::new(), false)), cv: Condvar::new() });
        let s2 = shared.clone();
        let h = thread::spawn(move || {
            let mut fired = 0;
            let mut g = s2.queue.lock().unwrap();
            loop {
                if let Some(Reverse((_, id))) = g.0.pop() {
                    g.2[id] = true;
                    if let Some(w) = g.1[id].take() {
                        drop(g);
                        thread::sleep(Duration::from_millis(1));
                        w.wake();
                        fired += 1;
                        g = s2.queue.lock().unwrap();
                    }
                    continue;
                }
                if g.3 {
                    return fired;
                }
                g = s2.cv.wait(g).unwrap();
            }
        });
        (Timer(shared), h)
    }
    fn sleep(&self, ticks: u64) -> Sleep {
        let mut g = self.0.queue.lock().unwrap();
        let id = g.1.len();
        g.1.push(None);
        g.2.push(false);
        drop(g);
        Sleep { timer: self.clone(), id, ticks, registered: false }
    }
    fn shutdown(&self) {
        self.0.queue.lock().unwrap().3 = true;
        self.0.cv.notify_all();
    }
}

struct Sleep {
    timer: Timer,
    id: usize,
    ticks: u64,
    registered: bool,
}
impl Future for Sleep {
    type Output = u64;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u64> {
        let mut g = self.timer.0.queue.lock().unwrap();
        if g.2[self.id] {
            return Poll::Ready(self.ticks);
        }
        g.1[self.id] = Some(cx.waker().clone());
        if !self.registered {
            g.0.push(Reverse((self.ticks, self.id)));
            drop(g);
            self.registered = true;
            self.timer.0.cv.notify_one();
        }
        Poll::Pending
    }
}

async fn steps(timer: Timer, log: Arc<Mutex<Vec<String>>>) -> u64 {
    let mut total = 0;
    for t in [3u64, 1, 2] {
        let got = timer.sleep(t).await;
        log.lock().unwrap().push(format!("slept {}", got));
        total += got;
    }
    total
}

fn main() {
    let (timer, h) = Timer::start();
    let log = Arc::new(Mutex::new(Vec::new()));
    let (total, wakes) = block_on(steps(timer.clone(), log.clone()));
    println!("total {} wakes>=3 {}", total, wakes >= 3);
    for l in log.lock().unwrap().iter() {
        println!("{}", l);
    }
    let ((a, b), _) = block_on(async {
        let a = timer.sleep(5).await;
        let b = timer.sleep(7).await;
        (a, b)
    });
    println!("{} {}", a, b);
    timer.shutdown();
    println!("timer fired {}", h.join().unwrap());
}
