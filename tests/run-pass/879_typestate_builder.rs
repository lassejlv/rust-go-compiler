use std::marker::PhantomData;

struct Locked;
struct Unlocked;

struct Door<S> {
    name: String,
    opens: u32,
    _s: PhantomData<S>,
}

impl Door<Locked> {
    fn new(name: &str) -> Self { Door { name: name.to_string(), opens: 0, _s: PhantomData } }
    fn unlock(self, code: u32) -> Result<Door<Unlocked>, Door<Locked>> {
        if code == 42 {
            Ok(Door { name: self.name, opens: self.opens, _s: PhantomData })
        } else {
            Err(self)
        }
    }
}
impl Door<Unlocked> {
    fn open(&mut self) { self.opens += 1; }
    fn lock(self) -> Door<Locked> { Door { name: self.name, opens: self.opens, _s: PhantomData } }
}
impl<S> Door<S> {
    fn info(&self) -> String { format!("{} opened {} times", self.name, self.opens) }
}

trait StateName { const NAME: &'static str; }
impl StateName for Locked { const NAME: &'static str = "locked"; }
impl StateName for Unlocked { const NAME: &'static str = "unlocked"; }
impl<S: StateName> Door<S> {
    fn state(&self) -> &'static str { S::NAME }
}

// builder with required fields tracked in types
struct No;
struct Yes;
struct ReqBuilder<U, M> {
    url: Option<String>,
    method: Option<String>,
    headers: Vec<(String, String)>,
    _p: PhantomData<(U, M)>,
}
#[derive(Debug)]
struct Request { url: String, method: String, headers: Vec<(String, String)> }

impl ReqBuilder<No, No> {
    fn new() -> Self { ReqBuilder { url: None, method: None, headers: vec![], _p: PhantomData } }
}
impl<M> ReqBuilder<No, M> {
    fn url(self, u: &str) -> ReqBuilder<Yes, M> {
        ReqBuilder { url: Some(u.into()), method: self.method, headers: self.headers, _p: PhantomData }
    }
}
impl<U> ReqBuilder<U, No> {
    fn method(self, m: &str) -> ReqBuilder<U, Yes> {
        ReqBuilder { url: self.url, method: Some(m.into()), headers: self.headers, _p: PhantomData }
    }
}
impl<U, M> ReqBuilder<U, M> {
    fn header(mut self, k: &str, v: &str) -> Self { self.headers.push((k.into(), v.into())); self }
}
impl ReqBuilder<Yes, Yes> {
    fn build(self) -> Request {
        Request { url: self.url.unwrap(), method: self.method.unwrap(), headers: self.headers }
    }
}

struct Meters<T>(f64, PhantomData<T>);
struct Metric;
struct Imperial;
impl<T> Meters<T> { fn new(v: f64) -> Self { Meters(v, PhantomData) } }
impl Meters<Metric> { fn show(&self) -> String { format!("{:.1} m", self.0) } }
impl Meters<Imperial> { fn show(&self) -> String { format!("{:.1} ft", self.0 * 3.28084) } }

fn main() {
    let d = Door::new("front");
    println!("{} [{}]", d.info(), d.state());
    let d = match d.unlock(1) { Ok(_) => panic!(), Err(d) => { println!("wrong code"); d } };
    let mut u = d.unlock(42).ok().unwrap();
    u.open();
    u.open();
    println!("{} [{}]", u.info(), u.state());
    let l = u.lock();
    println!("{} [{}]", l.info(), l.state());
    let r = ReqBuilder::new().header("a", "1").method("GET").header("b", "2").url("http://x").build();
    println!("{:?}", r);
    let r2 = ReqBuilder::new().url("/y").method("POST").build();
    println!("{} {} {}", r2.method, r2.url, r2.headers.len());
    println!("{} {}", Meters::<Metric>::new(2.0).show(), Meters::<Imperial>::new(2.0).show());
    println!("{}", std::mem::size_of::<Door<Locked>>() == std::mem::size_of::<Door<Unlocked>>());
}
