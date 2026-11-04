use std::fmt::{self, Display};
use std::marker::PhantomData;
use std::ops::{Deref, DerefMut};
use std::rc::Rc;
use std::cell::RefCell;

mod sealed {
    pub trait Sealed {}
}

pub trait State: sealed::Sealed {
    const NAME: &'static str;
}
pub struct Empty;
pub struct HasUrl;
pub struct Ready;
impl sealed::Sealed for Empty {}
impl sealed::Sealed for HasUrl {}
impl sealed::Sealed for Ready {}
impl State for Empty { const NAME: &'static str = "empty"; }
impl State for HasUrl { const NAME: &'static str = "has-url"; }
impl State for Ready { const NAME: &'static str = "ready"; }

pub struct RequestBuilder<S: State> {
    url: Option<String>,
    method: &'static str,
    headers: Vec<(String, String)>,
    _s: PhantomData<S>,
}

impl RequestBuilder<Empty> {
    pub fn new() -> Self {
        RequestBuilder { url: None, method: "GET", headers: vec![], _s: PhantomData }
    }
    pub fn url(self, u: &str) -> RequestBuilder<HasUrl> {
        RequestBuilder { url: Some(u.to_string()), method: self.method, headers: self.headers, _s: PhantomData }
    }
}

impl<S: State> RequestBuilder<S> {
    pub fn header(mut self, k: &str, v: &str) -> Self {
        self.headers.push((k.into(), v.into()));
        self
    }
    pub fn state(&self) -> &'static str {
        S::NAME
    }
}

impl RequestBuilder<HasUrl> {
    pub fn method(self, m: &'static str) -> RequestBuilder<Ready> {
        RequestBuilder { url: self.url, method: m, headers: self.headers, _s: PhantomData }
    }
}

impl RequestBuilder<Ready> {
    pub fn build(self) -> Request {
        Request { line: format!("{} {}", self.method, self.url.unwrap()), headers: self.headers }
    }
}

pub struct Request {
    line: String,
    headers: Vec<(String, String)>,
}
impl Display for Request {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.line)?;
        for (k, v) in &self.headers {
            write!(f, "\n  {}: {}", k, v)?;
        }
        Ok(())
    }
}

// Blanket impls and method resolution
trait Describe {
    fn describe(&self) -> String;
}
impl<T: Display + ?Sized> Describe for T {
    fn describe(&self) -> String {
        format!("<<{}>>", self)
    }
}

trait Shout {
    fn shout(&self) -> String;
}
impl Shout for str {
    fn shout(&self) -> String { self.to_uppercase() }
}
impl<'a, T: Shout + ?Sized> Shout for &'a T {
    fn shout(&self) -> String { format!("&{}", (**self).shout()) }
}
impl<T: Shout + ?Sized> Shout for Box<T> {
    fn shout(&self) -> String { format!("box({})", (**self).shout()) }
}

trait Name { fn name(&self) -> &'static str; }
struct Dog;
impl Name for Dog { fn name(&self) -> &'static str { "Dog" } }
impl<'a> Name for &'a Dog { fn name(&self) -> &'static str { "&Dog" } }
impl<'a> Name for &'a mut Dog { fn name(&self) -> &'static str { "&mut Dog" } }

// Deref chains
struct Wrapper<T> { inner: T, reads: std::cell::Cell<u32> }
impl<T> Deref for Wrapper<T> {
    type Target = T;
    fn deref(&self) -> &T {
        self.reads.set(self.reads.get() + 1);
        &self.inner
    }
}
impl<T> DerefMut for Wrapper<T> {
    fn deref_mut(&mut self) -> &mut T { &mut self.inner }
}
struct Inner { v: Vec<i32> }
impl Inner {
    fn total(&self) -> i32 { self.v.iter().sum() }
    fn push(&mut self, x: i32) { self.v.push(x) }
}

fn len_of(s: &str) -> usize { s.len() }

fn main() {
    let b = RequestBuilder::new();
    println!("{}", b.state());
    let b = b.header("A", "1").url("http://x");
    println!("{}", b.state());
    let b = b.method("POST").header("B", "2");
    println!("{}", b.state());
    println!("{}", b.build());

    println!("{}", 42.describe());
    println!("{}", "str".describe());
    println!("{}", String::from("owned").describe());
    let r: &dyn Display = &3.5;
    println!("{}", r.describe());

    let s: &str = "hi";
    println!("{}", s.shout());
    println!("{}", (&s).shout());
    let bx: Box<str> = "yo".into();
    println!("{}", bx.shout());
    let bb: Box<&str> = Box::new("q");
    println!("{}", bb.shout());

    let mut d = Dog;
    println!("{}", d.name());
    println!("{}", (&d).name());
    println!("{}", (&mut d).name());
    println!("{}", Name::name(&&d));
    println!("{}", <&Dog as Name>::name(&&d));

    let mut w = Wrapper { inner: Wrapper { inner: Inner { v: vec![1, 2] }, reads: Default::default() }, reads: Default::default() };
    w.push(10);
    println!("{} {}", w.total(), w.v.len());
    println!("reads outer={} inner={}", w.reads.get(), w.inner.reads.get());

    let rc = Rc::new(RefCell::new(Box::new(String::from("abc"))));
    rc.borrow_mut().push_str("def");
    println!("{} {}", len_of(&rc.borrow()), rc.borrow().to_uppercase());
    let nested: Box<Rc<Box<String>>> = Box::new(Rc::new(Box::new("deep".to_string())));
    println!("{} {}", len_of(&nested), nested.starts_with("de"));
    let ws = Wrapper { inner: String::from("wrapped"), reads: Default::default() };
    println!("{} {}", len_of(&ws), ws.reads.get());
}
