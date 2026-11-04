// edition:2018
// Edition 2018: async fn / async blocks driven by a hand-written executor,
// `r#try!` and other raw identifiers for new keywords, `pub(crate)` and
// `pub(in path)` visibility, `dyn` trait objects, and `impl Trait` arguments.
use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

mod outer {
    pub mod inner {
        pub(in crate::outer) fn secret() -> u32 { 41 }
        pub(crate) fn exposed() -> u32 { secret() + 1 }
    }
    pub fn via_parent() -> u32 { inner::secret() * 2 }
}

fn r#async(x: u32) -> u32 { x * 3 }
struct r#dyn { r#await: u32 }

fn checked(a: u32, b: u32) -> Result<u32, String> {
    let s = r#try!(a.checked_add(b).ok_or_else(|| format!("{} + {} overflows", a, b)));
    Ok(s)
}

/// A future that yields `n` times before completing.
struct YieldN { left: u32, log: Rc<RefCell<Vec<String>>>, label: &'static str }

impl Future for YieldN {
    type Output = u32;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<u32> {
        if self.left == 0 {
            self.log.borrow_mut().push(format!("{} ready", self.label));
            Poll::Ready(self.label.len() as u32)
        } else {
            self.left -= 1;
            self.log.borrow_mut().push(format!("{} pending ({} left)", self.label, self.left));
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}

async fn step(label: &'static str, n: u32, log: Rc<RefCell<Vec<String>>>) -> u32 {
    let v = YieldN { left: n, log: log.clone(), label }.await;
    log.borrow_mut().push(format!("{} got {}", label, v));
    v * 10
}

async fn pipeline(log: Rc<RefCell<Vec<String>>>) -> u32 {
    let a = step("alpha", 2, log.clone()).await;
    let b = async { step("beta", 1, log.clone()).await + 1 }.await;
    let c = {
        let log = log.clone();
        async move { step("gamma", 0, log).await }
    }
    .await;
    a + b + c
}

/// round-robin executor over boxed futures
fn run_all(mut tasks: VecDeque<(usize, Pin<Box<dyn Future<Output = u32>>>)>) -> Vec<(usize, u32, u32)> {
    let waker = Waker::noop();
    let mut cx = Context::from_waker(&waker);
    let mut done = Vec::new();
    let mut polls = vec![0u32; tasks.len()];
    while let Some((id, mut fut)) = tasks.pop_front() {
        polls[id] += 1;
        match fut.as_mut().poll(&mut cx) {
            Poll::Ready(v) => done.push((id, v, polls[id])),
            Poll::Pending => tasks.push_back((id, fut)),
        }
    }
    done
}

fn total(xs: impl IntoIterator<Item = u32>) -> u32 { xs.into_iter().sum() }

fn main() {
    println!("{} {} {}", outer::inner::exposed(), outer::via_parent(), r#async(5));
    let d = r#dyn { r#await: 9 };
    println!("raw field {}", d.r#await);
    println!("{:?} {:?}", checked(1, 2), checked(u32::MAX, 1));

    let log = Rc::new(RefCell::new(Vec::new()));
    let mut tasks: VecDeque<(usize, Pin<Box<dyn Future<Output = u32>>>)> = VecDeque::new();
    tasks.push_back((0, Box::pin(pipeline(log.clone()))));
    tasks.push_back((1, Box::pin(step("solo", 3, log.clone()))));
    let l2 = log.clone();
    tasks.push_back((2, Box::pin(async move {
        l2.borrow_mut().push("instant".to_string());
        7
    })));
    let results = run_all(tasks);
    for (id, v, polls) in &results {
        println!("task {} -> {} after {} polls", id, v, polls);
    }
    for (i, line) in log.borrow().iter().enumerate() {
        println!("{:>2} {}", i, line);
    }
    println!("total {}", total(results.iter().map(|r| r.1)));
    let shapes: Vec<Box<dyn Fn(f64) -> f64>> = vec![Box::new(|r| 3.0 * r * r), Box::new(|s| s * s)];
    println!("{:?}", shapes.iter().map(|f| f(2.0)).collect::<Vec<_>>());
}
