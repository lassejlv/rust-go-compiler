// A Stream trait with poll_next, an extension trait with async default methods,
// stream combinators, and a router of boxed async handlers (dyn Fn -> Pin<Box<dyn Future>>).
use std::collections::HashMap;
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

trait Stream {
    type Item;
    fn poll_next(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<Self::Item>>;
}

struct Iter<I> {
    it: I,
    flip: bool,
}
impl<I: Iterator + Unpin> Stream for Iter<I> {
    type Item = I::Item;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<I::Item>> {
        self.flip = !self.flip;
        if self.flip {
            cx.waker().wake_by_ref();
            return Poll::Pending;
        }
        Poll::Ready(self.it.next())
    }
}
fn iter<I: IntoIterator>(i: I) -> Iter<I::IntoIter> {
    Iter { it: i.into_iter(), flip: false }
}

struct MapS<S, F> {
    s: S,
    f: F,
}
impl<S: Stream + Unpin, F: FnMut(S::Item) -> T + Unpin, T> Stream for MapS<S, F> {
    type Item = T;
    fn poll_next(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Option<T>> {
        let this = &mut *self;
        match Pin::new(&mut this.s).poll_next(cx) {
            Poll::Ready(Some(v)) => Poll::Ready(Some((this.f)(v))),
            Poll::Ready(None) => Poll::Ready(None),
            Poll::Pending => Poll::Pending,
        }
    }
}

struct Next<'a, S: ?Sized>(&'a mut S);
impl<S: Stream + Unpin + ?Sized> Future for Next<'_, S> {
    type Output = Option<S::Item>;
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        Pin::new(&mut *self.0).poll_next(cx)
    }
}

trait StreamExt: Stream {
    fn next(&mut self) -> Next<'_, Self>
    where
        Self: Unpin,
    {
        Next(self)
    }
    fn map<F, T>(self, f: F) -> MapS<Self, F>
    where
        Self: Sized,
        F: FnMut(Self::Item) -> T,
    {
        MapS { s: self, f }
    }
    async fn collect_vec(mut self) -> Vec<Self::Item>
    where
        Self: Sized + Unpin,
    {
        let mut v = Vec::new();
        while let Some(x) = self.next().await {
            v.push(x);
        }
        v
    }
    async fn fold<A, F: FnMut(A, Self::Item) -> A>(mut self, init: A, mut f: F) -> A
    where
        Self: Sized + Unpin,
    {
        let mut acc = init;
        while let Some(x) = self.next().await {
            acc = f(acc, x);
        }
        acc
    }
}
impl<S: Stream + ?Sized> StreamExt for S {}

type Handler = Box<dyn Fn(String) -> Pin<Box<dyn Future<Output = (u16, String)>>>>;

struct Router {
    routes: HashMap<&'static str, Handler>,
}
impl Router {
    fn route<F, Fut>(&mut self, path: &'static str, f: F)
    where
        F: Fn(String) -> Fut + 'static,
        Fut: Future<Output = (u16, String)> + 'static,
    {
        self.routes.insert(path, Box::new(move |body| Box::pin(f(body))));
    }
    async fn handle(&self, path: &str, body: &str) -> (u16, String) {
        match self.routes.get(path) {
            Some(h) => h(body.to_string()).await,
            None => (404, format!("no route {}", path)),
        }
    }
}

async fn echo(body: String) -> (u16, String) {
    (200, body)
}

fn main() {
    let v = block_on(iter(1..=5).map(|x| x * x).collect_vec());
    println!("{:?}", v);
    let s = block_on(iter(vec!["a", "b", "c"]).map(str::to_uppercase).fold(String::new(), |mut a, x| {
        a.push_str(&x);
        a
    }));
    println!("{}", s);
    let mut st = iter([10, 20]);
    let r = block_on(async {
        let a = st.next().await;
        let b = st.next().await;
        let c = st.next().await;
        (a, b, c)
    });
    println!("{:?}", r);

    let mut router = Router { routes: HashMap::new() };
    router.route("/echo", echo);
    router.route("/len", |b: String| async move { (200, b.len().to_string()) });
    let prefix = String::from("hello, ");
    router.route("/greet", move |b: String| {
        let p = prefix.clone();
        async move { (201, p + &b) }
    });
    let reqs = [("/echo", "ping"), ("/len", "abcdef"), ("/greet", "world"), ("/nope", "")];
    for (p, b) in reqs {
        let (code, body) = block_on(router.handle(p, b));
        println!("{} {} -> {} {}", p, b, code, body);
    }
}
