// Async odds and ends: std::future::{ready, pending, poll_fn}, pin!, async fn
// argument drop timing, generic async fns, futures stored in a Vec and driven
// round-robin, async blocks borrowing locals, and Pin<&mut dyn Future>.
use std::cell::{Cell, RefCell};
use std::future::{self, Future};
use std::pin::{pin, Pin};
use std::rc::Rc;
use std::task::{Context, Poll, Waker};

thread_local! { static EV: RefCell<Vec<String>> = RefCell::new(vec![]); }
fn ev(s: String) { EV.with(|e| e.borrow_mut().push(s)); }
fn dump(t: &str) { let v = EV.with(|e| std::mem::take(&mut *e.borrow_mut())); println!("{:<12}{}", t, v.join(" ")); }

struct Arg(&'static str);
impl Drop for Arg { fn drop(&mut self) { ev(format!("~{}", self.0)); } }

fn run<F: Future>(f: F) -> F::Output {
    let mut f = pin!(f);
    let mut cx = Context::from_waker(Waker::noop());
    loop { if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { return v; } }
}

fn yield_now() -> impl Future<Output = ()> {
    let mut done = false;
    future::poll_fn(move |cx| if done { Poll::Ready(()) } else { done = true; cx.waker().wake_by_ref(); Poll::Pending })
}

async fn consume(a: Arg, b: Arg, n: u32) -> u32 {
    ev(format!("start {} {}", a.0, b.0));
    drop(b);
    for _ in 0..n { yield_now().await; }
    ev("end".into());
    n * 2
}

async fn ignore_arg(_: Arg, _unused: Arg) -> u8 { ev("body".into()); 1 }

async fn sum_all<I: IntoIterator<Item = F>, F: Future<Output = i64>>(futs: I) -> i64 {
    let mut t = 0;
    for f in futs { t += f.await; }
    t
}

async fn double_later(x: i64) -> i64 { yield_now().await; x * 2 }

fn round_robin(mut tasks: Vec<Pin<Box<dyn Future<Output = String>>>>) -> Vec<(usize, String, u32)> {
    let mut cx = Context::from_waker(Waker::noop());
    let mut done = vec![];
    let mut round = 0;
    let mut slots: Vec<Option<Pin<Box<dyn Future<Output = String>>>>> = tasks.drain(..).map(Some).collect();
    while done.len() < slots.len() {
        round += 1;
        for (i, s) in slots.iter_mut().enumerate() {
            if let Some(f) = s {
                if let Poll::Ready(v) = f.as_mut().poll(&mut cx) { done.push((i, v, round)); *s = None; }
            }
        }
    }
    done
}

fn poll_dyn(f: Pin<&mut dyn Future<Output = u8>>) -> Poll<u8> {
    let mut cx = Context::from_waker(Waker::noop());
    f.poll(&mut cx)
}

fn main() {
    println!("ready: {}", run(future::ready(41)) + 1);
    let r = run(consume(Arg("a"), Arg("b"), 3));
    ev(format!("r={}", r));
    dump("args:");
    let fut = ignore_arg(Arg("x"), Arg("y"));
    ev("created".into());
    let v = run(fut);
    ev(format!("v={}", v));
    dump("unused args:");

    let futs: Vec<_> = (1..=4).map(double_later).collect();
    println!("sum_all: {}", run(sum_all(futs)));
    println!("sum_all ready: {}", run(sum_all([future::ready(5), future::ready(6)])));

    let counter = Rc::new(Cell::new(0));
    let mk = |name: &'static str, steps: u32| -> Pin<Box<dyn Future<Output = String>>> {
        let c = counter.clone();
        Box::pin(async move {
            for _ in 0..steps { c.set(c.get() + 1); yield_now().await; }
            format!("{} after {}", name, steps)
        })
    };
    let res = round_robin(vec![mk("slow", 5), mk("fast", 1), mk("mid", 3), mk("instant", 0)]);
    println!("round robin: {:?} counter={}", res, counter.get());

    let mut data = vec![1, 2, 3];
    let total = run(async {
        let r = &mut data;
        r.push(4);
        yield_now().await;
        r.iter().sum::<i32>()
    });
    println!("borrowing block: total={} data={:?}", total, data);

    let mut p = pin!(async { yield_now().await; 7u8 });
    let first = poll_dyn(p.as_mut());
    let second = poll_dyn(p.as_mut());
    println!("dyn polls: {:?} {:?}", first, second);
    let mut pend = pin!(future::pending::<u8>());
    println!("pending: {:?}", poll_dyn(pend.as_mut()));

    let nested = run(async {
        let a = async { 1 };
        let b = async { yield_now().await; 2 };
        let c = async move { a.await + b.await };
        c.await * 10
    });
    println!("nested: {}", nested);
    let s = String::from("captured");
    let len = run(async { s.len() });
    println!("captured by ref: {} still usable: {}", len, s);
}
