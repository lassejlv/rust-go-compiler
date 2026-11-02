use std::cell::{Cell, RefCell};
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

// A deterministic executor with a virtual clock: tasks sleep until a tick.
struct Runtime {
    now: Cell<u64>,
    log: RefCell<Vec<String>>,
}

struct Sleep {
    rt: Rc<Runtime>,
    until: u64,
}

impl Future for Sleep {
    type Output = u64;
    fn poll(self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<u64> {
        let now = self.rt.now.get();
        if now >= self.until {
            Poll::Ready(now)
        } else {
            Poll::Pending
        }
    }
}

fn sleep(rt: &Rc<Runtime>, ticks: u64) -> Sleep {
    Sleep { rt: rt.clone(), until: rt.now.get() + ticks }
}

async fn ticker(rt: Rc<Runtime>, name: &'static str, period: u64, count: u32) -> u32 {
    let mut fired = 0;
    for _ in 0..count {
        let t = sleep(&rt, period).await;
        rt.log.borrow_mut().push(format!("t={} {}", t, name));
        fired += 1;
    }
    fired
}

fn run_all(rt: &Rc<Runtime>, tasks: Vec<Pin<Box<dyn Future<Output = u32>>>>) -> Vec<u32> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut slots: BTreeMap<usize, Pin<Box<dyn Future<Output = u32>>>> = tasks.into_iter().enumerate().collect();
    let mut results = vec![0; slots.len()];
    while !slots.is_empty() {
        let ids: Vec<usize> = slots.keys().copied().collect();
        for id in ids {
            let fut = slots.get_mut(&id).unwrap();
            if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
                results[id] = v;
                slots.remove(&id);
                rt.log.borrow_mut().push(format!("t={} task{} done", rt.now.get(), id));
            }
        }
        rt.now.set(rt.now.get() + 1);
        if rt.now.get() > 1000 {
            break;
        }
    }
    results
}

fn main() {
    let rt = Rc::new(Runtime { now: Cell::new(0), log: RefCell::new(Vec::new()) });
    let tasks: Vec<Pin<Box<dyn Future<Output = u32>>>> = vec![
        Box::pin(ticker(rt.clone(), "fast", 2, 4)),
        Box::pin(ticker(rt.clone(), "slow", 5, 2)),
        Box::pin({
            let rt = rt.clone();
            async move {
                let a = sleep(&rt, 3).await;
                let b = sleep(&rt, 3).await;
                rt.log.borrow_mut().push(format!("t={} once a={} b={}", rt.now.get(), a, b));
                99
            }
        }),
    ];
    let results = run_all(&rt, tasks);
    for line in rt.log.borrow().iter() {
        println!("{}", line);
    }
    println!("results {:?} final time {}", results, rt.now.get());
}
