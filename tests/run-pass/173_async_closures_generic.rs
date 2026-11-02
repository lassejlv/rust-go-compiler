use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn apply<F, Fut>(f: F, x: i32) -> i32
where
    F: Fn(i32) -> Fut,
    Fut: Future<Output = i32>,
{
    f(x).await + f(x + 1).await
}

async fn map_all<T, U, F, Fut>(items: Vec<T>, f: F) -> Vec<U>
where
    F: Fn(T) -> Fut,
    Fut: Future<Output = U>,
{
    let mut out = Vec::with_capacity(items.len());
    for it in items {
        out.push(f(it).await);
    }
    out
}

fn make_future(n: u32) -> impl Future<Output = String> {
    async move { "*".repeat(n as usize) }
}

async fn retry<F, Fut>(mut attempt: F, max: u32) -> Result<u32, String>
where
    F: FnMut(u32) -> Fut,
    Fut: Future<Output = Result<u32, String>>,
{
    let mut last = String::new();
    for i in 1..=max {
        match attempt(i).await {
            Ok(v) => return Ok(v),
            Err(e) => {
                println!("  attempt {} failed: {}", i, e);
                last = e;
            }
        }
    }
    Err(format!("gave up: {}", last))
}

struct Pipeline {
    stages: Vec<Box<dyn Fn(i64) -> Pin<Box<dyn Future<Output = i64>>>>>,
}

impl Pipeline {
    async fn run(&self, mut v: i64) -> i64 {
        for (i, s) in self.stages.iter().enumerate() {
            v = s(v).await;
            println!("  stage {} -> {}", i, v);
        }
        v
    }
}

fn main() {
    let r = block_on(apply(|x| async move { x * 10 }, 4));
    println!("apply = {}", r);
    let words = vec!["a", "bb", "ccc"];
    let lens = block_on(map_all(words, |w| async move { w.len() * 2 }));
    println!("{:?}", lens);
    println!("{}", block_on(make_future(5)));

    let threshold = 3;
    let r = block_on(retry(|i| async move { if i >= threshold { Ok(i * 100) } else { Err(format!("too early ({})", i)) } }, 5));
    println!("{:?}", r);
    let r = block_on(retry(|i| async move { Err::<u32, _>(format!("never {}", i)) }, 2));
    println!("{:?}", r);

    // async closures (Rust 1.85+)
    let base = 7;
    let add = async |x: i32| x + base;
    println!("async closure: {}", block_on(add(3)));
    let mut total = 0;
    block_on(async {
        for i in 1..=3 {
            total += add(i).await;
        }
    });
    println!("total {}", total);

    let p = Pipeline {
        stages: vec![
            Box::new(|v| Box::pin(async move { v + 1 })),
            Box::new(|v| Box::pin(async move { v * 3 })),
            Box::new(|v| Box::pin(async move { v - 4 })),
        ],
    };
    println!("pipeline {}", block_on(p.run(5)));
}
