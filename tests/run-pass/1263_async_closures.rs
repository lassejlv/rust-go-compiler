// async closures (Rust 2024 feature stabilized in 1.85) and AsyncFn* bounds.
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

async fn apply_twice<F: AsyncFn(i32) -> i32>(f: F, x: i32) -> i32 {
    let a = f(x).await;
    f(a).await
}

async fn call_mut<F: AsyncFnMut(&str)>(mut f: F) {
    f("one").await;
    f("two").await;
}

async fn call_once<F: AsyncFnOnce() -> String>(f: F) -> String {
    f().await
}

fn main() {
    let k = 5;
    let add_k = async |x: i32| x + k;
    println!("{}", block_on(apply_twice(add_k, 1)));

    let mut seen = Vec::new();
    block_on(call_mut(async |s: &str| {
        seen.push(s.len());
    }));
    println!("{:?}", seen);

    let owned = String::from("moved string");
    let r = block_on(call_once(async move || owned + "!"));
    println!("{}", r);

    // plain closure returning async block
    let make = |n: u32| async move { n * n };
    let mut total = 0;
    for i in 1..=4 {
        total += block_on(make(i));
    }
    println!("total {}", total);

    // async closure called directly
    let greet = async |name: String| format!("hello {}", name);
    println!("{}", block_on(greet("bob".to_string())));
    println!("{}", block_on(greet("amy".to_string())));
}
