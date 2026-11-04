// Generic async fns with trait bounds, async blocks returning closures, futures stored in
// structs/enums/Vec<Option<..>>, size_of futures, Send-ness checked at compile time,
// and futures moved between threads.
use std::fmt::Debug;
use std::future::Future;
use std::pin::Pin;
use std::sync::mpsc;
use std::task::{Context, Poll, Waker};
use std::thread;

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

struct Tick(u8);
impl Future for Tick {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 { Poll::Ready(()) } else { self.0 -= 1; Poll::Pending }
    }
}

async fn fold_async<T, A, F>(items: Vec<T>, init: A, f: F) -> A
where
    T: Debug,
    F: Fn(A, T) -> A,
{
    let mut acc = init;
    for it in items {
        Tick(1).await;
        acc = f(acc, it);
    }
    acc
}

async fn make_adder(n: i32) -> impl Fn(i32) -> i32 {
    Tick(2).await;
    move |x| x + n
}

fn assert_send<T: Send>(t: T) -> T {
    t
}

async fn sendable(v: Vec<String>) -> usize {
    let mut n = 0;
    for s in &v {
        Tick(1).await;
        n += s.len();
    }
    n
}

enum Stage<F: Future> {
    Running(Pin<Box<F>>),
    Done(F::Output),
}

fn main() {
    let r = block_on(fold_async(vec![1, 2, 3, 4], String::new(), |mut acc, x| {
        acc.push_str(&x.to_string());
        acc
    }));
    println!("{}", r);
    let r = block_on(fold_async(vec!["a", "bb"], 0usize, |acc, s| acc + s.len()));
    println!("{}", r);
    let add5 = block_on(make_adder(5));
    println!("{}", add5(10));

    let fut = assert_send(sendable(vec!["abc".into(), "de".into()]));
    let (tx, rx) = mpsc::channel();
    let h = thread::spawn(move || {
        let v = block_on(fut);
        tx.send(v).unwrap();
    });
    h.join().unwrap();
    println!("from thread {}", rx.recv().unwrap());

    let futs: Vec<_> = (0..4u8).map(|i| async move { Tick(i).await; i * 2 }).collect();
    let mut stages: Vec<Stage<_>> = futs.into_iter().map(|f| Stage::Running(Box::pin(f))).collect();
    let mut cx = Context::from_waker(Waker::noop());
    let mut rounds = 0;
    while stages.iter().any(|s| matches!(s, Stage::Running(_))) {
        rounds += 1;
        for s in stages.iter_mut() {
            if let Stage::Running(f) = s {
                if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                    *s = Stage::Done(v);
                }
            }
        }
    }
    let outs: Vec<u8> = stages.into_iter().map(|s| match s { Stage::Done(v) => v, _ => 255 }).collect();
    println!("rounds {} outs {:?}", rounds, outs);

    let small = async { 1u8 };
    let big = async {
        let buf = [0u8; 256];
        Tick(1).await;
        buf.len()
    };
    println!("{} {}", block_on(small), block_on(big));
    let mut opt: Option<Pin<Box<dyn Future<Output = i32>>>> = Some(Box::pin(async { Tick(3).await; 99 }));
    let mut n = 0;
    loop {
        n += 1;
        if let Some(f) = opt.as_mut() {
            if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
                println!("opt ready {} after {}", v, n);
                opt = None;
            }
        } else {
            break;
        }
    }
}
