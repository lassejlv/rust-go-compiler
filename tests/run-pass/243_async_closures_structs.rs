// Closures returning futures stored in structs (retry policies, task factories), async closures
// (Rust 2021 `async ||` via AsyncFn traits), and generic async callbacks.
use std::cell::RefCell;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn block_on<F: Future>(f: F) -> F::Output {
    let w = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&w);
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

type Factory<T> = Box<dyn Fn(u32) -> Pin<Box<dyn Future<Output = Result<T, String>>>>>;

struct Retry<T> {
    name: &'static str,
    max_attempts: u32,
    op: Factory<T>,
}

impl<T: std::fmt::Debug> Retry<T> {
    async fn run(&self) -> Result<T, String> {
        let mut last = String::new();
        for attempt in 1..=self.max_attempts {
            match (self.op)(attempt).await {
                Ok(v) => {
                    println!("  {}: attempt {} ok {:?}", self.name, attempt, v);
                    return Ok(v);
                }
                Err(e) => {
                    println!("  {}: attempt {} failed: {}", self.name, attempt, e);
                    last = e;
                }
            }
        }
        Err(format!("{} gave up: {}", self.name, last))
    }
}

// Generic struct holding a closure that returns a future
struct Pipeline<F> {
    stages: Vec<F>,
}

impl<F, Fut> Pipeline<F>
where
    F: Fn(i64) -> Fut,
    Fut: Future<Output = i64>,
{
    async fn run(&self, mut x: i64) -> i64 {
        for s in &self.stages {
            x = s(x).await;
        }
        x
    }
}

async fn apply_twice<F>(f: F, x: i32) -> i32
where
    F: AsyncFn(i32) -> i32,
{
    let a = f(x).await;
    f(a).await
}

async fn visit_all<F>(items: &[&str], mut f: F)
where
    F: AsyncFnMut(&str),
{
    for i in items {
        f(i).await;
    }
}

async fn double(x: i64) -> i64 {
    x * 2
}

fn main() {
    let calls = Rc::new(RefCell::new(0));
    let c2 = calls.clone();
    let flaky = Retry {
        name: "flaky",
        max_attempts: 5,
        op: Box::new(move |attempt| {
            *c2.borrow_mut() += 1;
            Box::pin(async move {
                if attempt < 3 {
                    Err(format!("transient #{}", attempt))
                } else {
                    Ok(attempt * 100)
                }
            })
        }),
    };
    let broken: Retry<String> = Retry {
        name: "broken",
        max_attempts: 2,
        op: Box::new(|a| Box::pin(async move { Err(format!("always fails ({})", a)) })),
    };
    println!("flaky -> {:?}", block_on(flaky.run()));
    println!("broken -> {:?}", block_on(broken.run()));
    println!("factory calls {}", calls.borrow());

    let p = Pipeline { stages: vec![double, double, double] };
    println!("pipeline(5) = {}", block_on(p.run(5)));
    let offset = 7;
    let p2 = Pipeline {
        stages: (1..=3).map(|k| move |x: i64| async move { x * k + offset }).collect::<Vec<_>>(),
    };
    println!("pipeline2(1) = {}", block_on(p2.run(1)));

    let base = 10;
    let add_base = async |x: i32| x + base;
    println!("apply_twice = {}", block_on(apply_twice(add_base, 1)));
    println!("apply_twice sq = {}", block_on(apply_twice(async |x: i32| x * x, 3)));

    let mut seen = Vec::new();
    block_on(visit_all(&["x", "yy", "zzz"], async |s: &str| {
        seen.push(format!("{}:{}", s, s.len()));
    }));
    println!("{:?}", seen);

    let handlers: Vec<(&str, Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = String>>>>)> = vec![
        ("rev", Box::new(|s| Box::pin(async move { s.chars().rev().collect() }))),
        ("dup", Box::new(|s| Box::pin(async move { format!("{0}{0}", s) }))),
    ];
    for (n, h) in &handlers {
        println!("{} -> {}", n, block_on(h("abc".to_string())));
    }
}
