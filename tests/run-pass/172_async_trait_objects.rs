use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

trait Handler {
    fn name(&self) -> &str;
    fn handle<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<String, String>>;
}

struct Upper;
struct Counter {
    prefix: String,
}
struct Failing;

impl Handler for Upper {
    fn name(&self) -> &str {
        "upper"
    }
    fn handle<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move { Ok(input.to_uppercase()) })
    }
}

impl Handler for Counter {
    fn name(&self) -> &str {
        "counter"
    }
    fn handle<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            let n = count_words(input).await;
            Ok(format!("{}{}", self.prefix, n))
        })
    }
}

impl Handler for Failing {
    fn name(&self) -> &str {
        "failing"
    }
    fn handle<'a>(&'a self, input: &'a str) -> BoxFuture<'a, Result<String, String>> {
        Box::pin(async move {
            if input.is_empty() {
                Ok("empty ok".to_string())
            } else {
                Err(format!("cannot handle {:?}", input))
            }
        })
    }
}

async fn count_words(s: &str) -> usize {
    s.split_whitespace().count()
}

// Native async fn in trait (Rust 1.75+), used with static dispatch.
trait Storage {
    async fn put(&mut self, k: &str, v: i32);
    async fn get(&self, k: &str) -> Option<i32>;
}

struct MemStore {
    map: BTreeMap<String, i32>,
}

impl Storage for MemStore {
    async fn put(&mut self, k: &str, v: i32) {
        self.map.insert(k.to_string(), v);
    }
    async fn get(&self, k: &str) -> Option<i32> {
        self.map.get(k).copied()
    }
}

async fn use_store<S: Storage>(s: &mut S) -> i32 {
    s.put("a", 1).await;
    s.put("b", 41).await;
    s.get("a").await.unwrap_or(0) + s.get("b").await.unwrap_or(0) + s.get("zzz").await.unwrap_or(1000)
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = Box::pin(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn main() {
    let handlers: Vec<Box<dyn Handler>> = vec![Box::new(Upper), Box::new(Counter { prefix: "words=".into() }), Box::new(Failing)];
    for input in ["hello async world", ""] {
        for h in &handlers {
            let r = block_on(h.handle(input));
            println!("{} {:?} -> {:?}", h.name(), input, r);
        }
    }
    let all = block_on(async {
        let mut out = Vec::new();
        for h in handlers.iter() {
            match h.handle("a b").await {
                Ok(s) => out.push(s),
                Err(e) => out.push(format!("ERR({})", e.len())),
            }
        }
        out
    });
    println!("{:?}", all);
    let mut store = MemStore { map: BTreeMap::new() };
    println!("store result {}", block_on(use_store(&mut store)));
    println!("{:?}", store.map);

    let closures: Vec<Box<dyn Fn(i32) -> BoxFuture<'static, i32>>> = vec![
        Box::new(|x| Box::pin(async move { x + 1 })),
        Box::new(|x| Box::pin(async move { x * x })),
    ];
    let outs: Vec<i32> = closures.iter().map(|f| block_on(f(7))).collect();
    println!("{:?}", outs);
}
