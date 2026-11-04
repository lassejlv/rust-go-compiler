// async fn in traits, -> impl Future in traits, generic dispatch over them.
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

struct Yield(u8);
impl Future for Yield {
    type Output = ();
    fn poll(mut self: Pin<&mut Self>, _cx: &mut Context<'_>) -> Poll<()> {
        if self.0 == 0 { Poll::Ready(()) } else { self.0 -= 1; Poll::Pending }
    }
}

trait Store {
    async fn get(&self, key: &str) -> Option<String>;
    async fn put(&mut self, key: &str, val: String);
    fn len_later(&self) -> impl Future<Output = usize> + '_;
    async fn get_or(&self, key: &str, default: &str) -> String {
        match self.get(key).await {
            Some(v) => v,
            None => default.to_string(),
        }
    }
}

struct VecStore {
    items: Vec<(String, String)>,
}

impl Store for VecStore {
    async fn get(&self, key: &str) -> Option<String> {
        Yield(2).await;
        self.items.iter().find(|(k, _)| k == key).map(|(_, v)| v.clone())
    }
    async fn put(&mut self, key: &str, val: String) {
        Yield(1).await;
        if let Some(e) = self.items.iter_mut().find(|(k, _)| k == key) {
            e.1 = val;
        } else {
            self.items.push((key.to_string(), val));
        }
    }
    fn len_later(&self) -> impl Future<Output = usize> + '_ {
        async move {
            Yield(1).await;
            self.items.len()
        }
    }
}

struct Upper<S>(S);
impl<S: Store> Store for Upper<S> {
    async fn get(&self, key: &str) -> Option<String> {
        self.0.get(key).await.map(|s| s.to_uppercase())
    }
    async fn put(&mut self, key: &str, val: String) {
        self.0.put(key, val).await
    }
    fn len_later(&self) -> impl Future<Output = usize> + '_ {
        let f = self.0.len_later();
        async move { f.await * 100 }
    }
}

async fn exercise<S: Store>(s: &mut S) -> Vec<String> {
    s.put("a", "apple".into()).await;
    s.put("b", "banana".into()).await;
    s.put("a", "avocado".into()).await;
    let mut out = vec![];
    out.push(s.get_or("a", "?").await);
    out.push(s.get_or("b", "?").await);
    out.push(s.get_or("c", "none").await);
    out.push(format!("len={}", s.len_later().await));
    out
}

trait Shape {
    fn area(&self) -> impl Future<Output = f64>;
}
struct Sq(f64);
struct Circ(f64);
impl Shape for Sq {
    fn area(&self) -> impl Future<Output = f64> {
        let s = self.0;
        async move { s * s }
    }
}
impl Shape for Circ {
    async fn area(&self) -> f64 {
        3.0 * self.0 * self.0
    }
}

async fn total<A: Shape, B: Shape>(a: &A, b: &B) -> f64 {
    a.area().await + b.area().await
}

fn main() {
    let mut s = VecStore { items: vec![] };
    println!("{:?}", block_on(exercise(&mut s)));
    let mut u = Upper(VecStore { items: vec![] });
    println!("{:?}", block_on(exercise(&mut u)));
    println!("{}", block_on(total(&Sq(2.0), &Circ(1.5))));
}
