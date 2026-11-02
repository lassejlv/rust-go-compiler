use std::fmt;
use std::marker::PhantomData;

// Type-state markers
struct NoUrl;
struct HasUrl;
struct NoMethod;
struct HasMethod;

#[derive(Debug, Clone, Copy, PartialEq)]
enum Method {
    Get,
    Post,
    Put,
}

struct RequestBuilder<U, M> {
    url: Option<String>,
    method: Option<Method>,
    headers: Vec<(String, String)>,
    body: Option<String>,
    timeout_ms: u32,
    _state: PhantomData<(U, M)>,
}

impl RequestBuilder<NoUrl, NoMethod> {
    fn new() -> Self {
        RequestBuilder {
            url: None,
            method: None,
            headers: Vec::new(),
            body: None,
            timeout_ms: 30_000,
            _state: PhantomData,
        }
    }
}

impl<M> RequestBuilder<NoUrl, M> {
    fn url(self, url: impl Into<String>) -> RequestBuilder<HasUrl, M> {
        RequestBuilder {
            url: Some(url.into()),
            method: self.method,
            headers: self.headers,
            body: self.body,
            timeout_ms: self.timeout_ms,
            _state: PhantomData,
        }
    }
}

impl<U> RequestBuilder<U, NoMethod> {
    fn method(self, method: Method) -> RequestBuilder<U, HasMethod> {
        RequestBuilder {
            url: self.url,
            method: Some(method),
            headers: self.headers,
            body: self.body,
            timeout_ms: self.timeout_ms,
            _state: PhantomData,
        }
    }
}

impl<U, M> RequestBuilder<U, M> {
    fn header(mut self, k: &str, v: &str) -> Self {
        self.headers.push((k.to_string(), v.to_string()));
        self
    }
    fn timeout(mut self, ms: u32) -> Self {
        self.timeout_ms = ms;
        self
    }
    fn body<B: ToString>(mut self, b: B) -> Self {
        self.body = Some(b.to_string());
        self
    }
}

impl RequestBuilder<HasUrl, HasMethod> {
    fn build(self) -> Request {
        Request {
            url: self.url.unwrap(),
            method: self.method.unwrap(),
            headers: self.headers,
            body: self.body,
            timeout_ms: self.timeout_ms,
        }
    }
}

#[derive(Debug)]
struct Request {
    url: String,
    method: Method,
    headers: Vec<(String, String)>,
    body: Option<String>,
    timeout_ms: u32,
}

impl fmt::Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{:?} {}", self.method, self.url)?;
        for (k, v) in &self.headers {
            write!(f, "\n  {}: {}", k, v)?;
        }
        if let Some(b) = &self.body {
            write!(f, "\n  <{} bytes body>", b.len())?;
        }
        write!(f, "\n  timeout={}ms", self.timeout_ms)
    }
}

// A second type-state machine: a connection that must be opened before use.
struct Closed;
struct Open;

struct Conn<S> {
    name: &'static str,
    sent: Vec<String>,
    _s: PhantomData<S>,
}

impl Conn<Closed> {
    fn new(name: &'static str) -> Self {
        Conn { name, sent: vec![], _s: PhantomData }
    }
    fn open(self) -> Conn<Open> {
        println!("[{}] opened", self.name);
        Conn { name: self.name, sent: self.sent, _s: PhantomData }
    }
}

impl Conn<Open> {
    fn send(&mut self, msg: &str) -> &mut Self {
        self.sent.push(msg.to_string());
        self
    }
    fn close(self) -> (Conn<Closed>, usize) {
        let n = self.sent.len();
        println!("[{}] closed after {} messages", self.name, n);
        (Conn { name: self.name, sent: self.sent, _s: PhantomData }, n)
    }
}

trait StateName {
    const NAME: &'static str;
}
impl StateName for Open {
    const NAME: &'static str = "open";
}
impl StateName for Closed {
    const NAME: &'static str = "closed";
}

fn describe<S: StateName>(c: &Conn<S>) -> String {
    format!("{} is {} ({} sent)", c.name, S::NAME, c.sent.len())
}

fn main() {
    let r = RequestBuilder::new()
        .header("Accept", "text/plain")
        .url("https://example.com/api")
        .timeout(500)
        .method(Method::Post)
        .body("{\"x\":1}")
        .header("X-Trace", "abc")
        .build();
    println!("{}", r);
    println!("{:?}", r.method == Method::Post);

    let r2 = RequestBuilder::new().method(Method::Get).url(String::from("/index")).build();
    println!("{}", r2);
    let r3 = RequestBuilder::new().url("/put").method(Method::Put).body(42).build();
    println!("{:?}", r3);

    let c = Conn::new("db");
    println!("{}", describe(&c));
    let mut c = c.open();
    c.send("hello").send("world");
    println!("{}", describe(&c));
    let (c, n) = c.close();
    println!("{} -> {}", describe(&c), n);
    println!("sizes: {} {}", std::mem::size_of::<PhantomData<(NoUrl, HasMethod)>>(), std::mem::size_of::<Closed>());
}
