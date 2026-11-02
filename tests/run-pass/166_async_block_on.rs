use std::future::Future;
use std::pin::Pin;
use std::ptr;
use std::task::{Context, Poll, RawWaker, RawWakerVTable, Waker};

fn noop_raw_waker() -> RawWaker {
    fn clone(_: *const ()) -> RawWaker {
        noop_raw_waker()
    }
    fn noop(_: *const ()) {}
    static VTABLE: RawWakerVTable = RawWakerVTable::new(clone, noop, noop, noop);
    RawWaker::new(ptr::null(), &VTABLE)
}

fn block_on<F: Future>(fut: F) -> F::Output {
    let waker = unsafe { Waker::from_raw(noop_raw_waker()) };
    let mut cx = Context::from_waker(&waker);
    let mut fut = Box::pin(fut);
    let mut polls = 0;
    loop {
        polls += 1;
        if let Poll::Ready(v) = fut.as_mut().poll(&mut cx) {
            println!("  (ready after {} poll(s))", polls);
            return v;
        }
    }
}

async fn double(x: i32) -> i32 {
    x * 2
}

async fn add_then_double(a: i32, b: i32) -> i32 {
    let s = a + b;
    double(s).await
}

async fn chain(n: i32) -> String {
    let mut acc = n;
    for i in 0..3 {
        acc = add_then_double(acc, i).await;
        println!("  step {} acc {}", i, acc);
    }
    format!("result {}", acc)
}

struct Greeter {
    name: String,
}

impl Greeter {
    async fn greet(&self, greeting: &str) -> String {
        format!("{}, {}!", greeting, self.name)
    }
}

async fn maybe_fail(x: i32) -> Result<i32, String> {
    if x < 0 {
        Err(format!("negative {}", x))
    } else {
        Ok(x + 1)
    }
}

async fn try_chain(x: i32) -> Result<i32, String> {
    let a = maybe_fail(x).await?;
    let b = maybe_fail(a - 5).await?;
    Ok(a * b)
}

fn main() {
    println!("{}", block_on(double(21)));
    println!("{}", block_on(chain(1)));
    let g = Greeter { name: "async world".to_string() };
    println!("{}", block_on(g.greet("hello")));
    println!("{:?}", block_on(try_chain(10)));
    println!("{:?}", block_on(try_chain(2)));
    println!("{:?}", block_on(try_chain(-1)));
    let v = block_on(async {
        let mut v = Vec::new();
        for i in 1..=4 {
            v.push(double(i).await);
        }
        v
    });
    println!("{:?}", v);
    let ready = block_on(std::future::ready("ready value"));
    println!("{}", ready);
}
