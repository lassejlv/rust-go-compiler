// Pin<Box<dyn Future<Output = T> + '_>> trait objects: middleware chains, async trait-object methods
// with borrowed self, and a router storing boxed async handlers.
use std::collections::BTreeMap;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll, Wake, Waker};

type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + 'a>>;

struct Noop;
impl Wake for Noop {
    fn wake(self: Arc<Self>) {}
}

fn block_on<T>(mut f: BoxFuture<'_, T>) -> T {
    let w = Waker::from(Arc::new(Noop));
    let mut cx = Context::from_waker(&w);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

#[derive(Debug, Clone)]
struct Request {
    path: String,
    body: String,
}

#[derive(Debug)]
struct Response {
    status: u16,
    body: String,
}

trait Handler {
    fn call<'a>(&'a self, req: Request) -> BoxFuture<'a, Response>;
}

// Wrap any async fn/closure as a Handler.
struct FnHandler<F>(F);
impl<F, Fut> Handler for FnHandler<F>
where
    F: Fn(Request) -> Fut,
    Fut: Future<Output = Response> + 'static,
{
    fn call<'a>(&'a self, req: Request) -> BoxFuture<'a, Response> {
        Box::pin((self.0)(req))
    }
}

struct Logger<H> {
    inner: H,
    prefix: String,
}
impl<H: Handler> Handler for Logger<H> {
    fn call<'a>(&'a self, req: Request) -> BoxFuture<'a, Response> {
        Box::pin(async move {
            let path = req.path.clone();
            let resp = self.inner.call(req).await;
            Response { status: resp.status, body: format!("{}[{} {}] {}", self.prefix, path, resp.status, resp.body) }
        })
    }
}

struct Auth<H> {
    inner: H,
    token: &'static str,
}
impl<H: Handler> Handler for Auth<H> {
    fn call<'a>(&'a self, req: Request) -> BoxFuture<'a, Response> {
        if !req.body.starts_with(self.token) {
            return Box::pin(std::future::ready(Response { status: 401, body: "unauthorized".into() }));
        }
        let stripped = Request { body: req.body[self.token.len()..].to_string(), ..req };
        self.inner.call(stripped)
    }
}

struct Router {
    routes: BTreeMap<String, Box<dyn Handler>>,
}

impl Router {
    fn route(mut self, p: &str, h: impl Handler + 'static) -> Self {
        self.routes.insert(p.to_string(), Box::new(h));
        self
    }
    fn dispatch(&self, req: Request) -> BoxFuture<'_, Response> {
        match self.routes.get(&req.path) {
            Some(h) => h.call(req),
            None => Box::pin(async move { Response { status: 404, body: format!("no route {}", req.path) } }),
        }
    }
}

async fn echo(req: Request) -> Response {
    Response { status: 200, body: req.body }
}

async fn count(req: Request) -> Response {
    let n = sub_count(&req.body).await;
    Response { status: 200, body: format!("{} words", n) }
}

async fn sub_count(s: &str) -> usize {
    s.split_whitespace().count()
}

fn boxed_chain<'a>(values: &'a [i32]) -> Vec<BoxFuture<'a, i32>> {
    values
        .iter()
        .map(|v| -> BoxFuture<'a, i32> {
            if *v % 2 == 0 {
                Box::pin(async move { *v / 2 })
            } else {
                Box::pin(std::future::ready(*v * 3 + 1))
            }
        })
        .collect()
}

fn main() {
    let router = Router { routes: BTreeMap::new() }
        .route("/echo", FnHandler(echo))
        .route("/count", Logger { inner: FnHandler(count), prefix: "LOG ".into() })
        .route("/secret", Logger { inner: Auth { inner: FnHandler(echo), token: "tok:" }, prefix: "".into() })
        .route(
            "/upper",
            FnHandler(|r: Request| async move { Response { status: 201, body: r.body.to_uppercase() } }),
        );
    let reqs = [
        ("/echo", "hello"),
        ("/count", "a b c d"),
        ("/secret", "tok:the payload"),
        ("/secret", "bad"),
        ("/upper", "shout"),
        ("/missing", ""),
    ];
    for (p, b) in reqs {
        let resp = block_on(router.dispatch(Request { path: p.into(), body: b.into() }));
        println!("{:<8} -> {} {:?}", p, resp.status, resp.body);
    }

    let vals = [6, 7, 10, 1];
    let futs = boxed_chain(&vals);
    let outs: Vec<i32> = futs.into_iter().map(block_on).collect();
    println!("{:?}", outs);

    let mut pending: Vec<(&str, BoxFuture<'static, String>)> = vec![
        ("a", Box::pin(async { "first".to_string() })),
        ("b", Box::pin(async { format!("{}-{}", "sec", 2) })),
    ];
    pending.reverse();
    for (k, f) in pending {
        println!("{} => {}", k, block_on(f));
    }
}
