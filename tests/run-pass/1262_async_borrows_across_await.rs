// Async fns/blocks holding borrows (shared and mutable) across await points.
use std::future::Future;
use std::pin::Pin;
use std::task::{Context, Poll, Waker};

struct Yield(bool);
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<()> {
        if self.0 {
            Poll::Ready(())
        } else {
            self.0 = true;
            cx.waker().wake_by_ref();
            Poll::Pending
        }
    }
}
fn yield_now() -> Yield {
    Yield(false)
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = std::pin::pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

async fn sum_slice(xs: &[i64]) -> i64 {
    let mut total = 0;
    for x in xs {
        yield_now().await;
        total += x;
    }
    total
}

async fn push_all(v: &mut Vec<String>, words: &[&str]) -> usize {
    for w in words {
        v.push(w.to_uppercase());
        yield_now().await;
    }
    v.len()
}

struct Counter {
    hits: u32,
    name: String,
}

impl Counter {
    async fn bump(&mut self, by: u32) -> u32 {
        yield_now().await;
        self.hits += by;
        self.hits
    }
    async fn describe(&self) -> String {
        let n = &self.name;
        yield_now().await;
        format!("{}:{}", n, self.hits)
    }
}

async fn longest<'a>(a: &'a str, b: &'a str) -> &'a str {
    yield_now().await;
    if a.len() >= b.len() { a } else { b }
}

fn main() {
    let data = vec![1, 2, 3, 4, 5];
    println!("sum={}", block_on(sum_slice(&data)));
    let mut v = vec!["start".to_string()];
    let n = block_on(push_all(&mut v, &["a", "bc", "def"]));
    println!("n={} v={:?}", n, v);
    let mut c = Counter { hits: 0, name: "ctr".into() };
    let r = block_on(async {
        let a = c.bump(3).await;
        let b = c.bump(4).await;
        let d = c.describe().await;
        (a, b, d)
    });
    println!("{:?}", r);
    let s1 = String::from("hello");
    let s2 = String::from("hi");
    println!("{}", block_on(longest(&s1, &s2)));
    let mut map = std::collections::BTreeMap::new();
    block_on(async {
        for (i, w) in ["x", "y", "x", "z", "x"].iter().enumerate() {
            let e = map.entry(*w).or_insert(Vec::new());
            yield_now().await;
            e.push(i);
        }
    });
    println!("{:?}", map);
    // async move capturing and returning ownership
    let owned = vec![10u8, 20, 30];
    let fut = async move {
        let r = &owned;
        yield_now().await;
        r.iter().map(|&x| x as u32).sum::<u32>() + owned.len() as u32
    };
    println!("{}", block_on(fut));
}
