// Typestate pattern: a connection whose states are zero-sized marker types
// with PhantomData; transitions consume self; shared methods via a generic
// impl, state-specific impls, sealed traits for valid states, a builder with
// required fields enforced by type parameters, and runtime simulation.
use std::fmt;
use std::marker::PhantomData;

mod sealed { pub trait Sealed {} }
trait State: sealed::Sealed { const NAME: &'static str; }
#[derive(Debug)] struct Closed;
#[derive(Debug)] struct Handshaking { attempts: u8 }
#[derive(Debug)] struct Open { session: u64 }
#[derive(Debug)] struct Draining { pending: usize }
impl sealed::Sealed for Closed {}
impl sealed::Sealed for Handshaking {}
impl sealed::Sealed for Open {}
impl sealed::Sealed for Draining {}
impl State for Closed { const NAME: &'static str = "closed"; }
impl State for Handshaking { const NAME: &'static str = "handshaking"; }
impl State for Open { const NAME: &'static str = "open"; }
impl State for Draining { const NAME: &'static str = "draining"; }

struct Conn<S: State> { peer: String, log: Vec<String>, state: S, sent: Vec<String> }

impl<S: State> Conn<S> {
    fn state_name(&self) -> &'static str { S::NAME }
    fn note(&mut self, m: impl Into<String>) { let m = m.into(); self.log.push(format!("[{}] {}", S::NAME, m)); }
    fn transition<T: State>(self, state: T) -> Conn<T> { let mut c = Conn { peer: self.peer, log: self.log, state, sent: self.sent }; c.note(format!("entered from {}", S::NAME)); c }
}
impl Conn<Closed> {
    fn new(peer: &str) -> Self { Conn { peer: peer.into(), log: vec![], state: Closed, sent: vec![] } }
    fn connect(self) -> Conn<Handshaking> { self.transition(Handshaking { attempts: 0 }) }
}
impl Conn<Handshaking> {
    fn attempt(mut self, ok: bool) -> Result<Conn<Open>, Conn<Handshaking>> {
        self.state.attempts += 1;
        let n = self.state.attempts;
        if ok { self.note(format!("succeeded on attempt {}", n)); Ok(self.transition(Open { session: 1000 + n as u64 * 7 })) }
        else { self.note(format!("attempt {} failed", n)); Err(self) }
    }
    fn give_up(self) -> Conn<Closed> { self.transition(Closed) }
}
impl Conn<Open> {
    fn send(&mut self, msg: &str) -> usize { self.sent.push(format!("{}#{}", self.state.session, msg)); self.sent.len() }
    fn close(self) -> Conn<Draining> { let p = self.sent.len(); self.transition(Draining { pending: p }) }
}
impl Conn<Draining> {
    fn flush_one(mut self) -> Result<Conn<Closed>, Conn<Draining>> {
        self.state.pending -= 1;
        if self.state.pending == 0 { Ok(self.transition(Closed)) } else { let p = self.state.pending; self.note(format!("{} pending", p)); Err(self) }
    }
}
impl<S: State + fmt::Debug> fmt::Display for Conn<S> {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result { write!(f, "Conn({} @ {:?}, sent={})", self.peer, self.state, self.sent.len()) }
}

struct Missing;
struct Present<T>(T);
struct ReqBuilder<U, M> { url: U, method: M, headers: Vec<(String, String)>, _p: PhantomData<()> }
impl ReqBuilder<Missing, Missing> { fn new() -> Self { ReqBuilder { url: Missing, method: Missing, headers: vec![], _p: PhantomData } } }
impl<U, M> ReqBuilder<U, M> {
    fn url(self, u: &str) -> ReqBuilder<Present<String>, M> { ReqBuilder { url: Present(u.into()), method: self.method, headers: self.headers, _p: PhantomData } }
    fn method(self, m: &'static str) -> ReqBuilder<U, Present<&'static str>> { ReqBuilder { url: self.url, method: Present(m), headers: self.headers, _p: PhantomData } }
    fn header(mut self, k: &str, v: &str) -> Self { self.headers.push((k.into(), v.into())); self }
}
impl ReqBuilder<Present<String>, Present<&'static str>> {
    fn build(self) -> String { format!("{} {} {:?}", self.method.0, self.url.0, self.headers) }
}

fn main() {
    let c = Conn::new("10.0.0.7:443");
    println!("{} state={}", c, c.state_name());
    let mut hs = c.connect();
    let mut open = None;
    for ok in [false, false, true] {
        match hs.attempt(ok) {
            Ok(o) => { open = Some(o); break; }
            Err(h) => { println!("retrying from {}", h.state_name()); hs = h; }
        }
    }
    let mut open = open.expect("connected");
    for m in ["hello", "data", "bye"] { let n = open.send(m); println!("sent {} total {}", m, n); }
    println!("{}", open);
    let mut dr = open.close();
    let closed = loop {
        match dr.flush_one() { Ok(c) => break c, Err(d) => dr = d }
    };
    println!("{} state={} sent={:?}", closed, closed.state_name(), closed.sent);
    for l in &closed.log { println!("  {}", l); }
    let failed = Conn::new("bad").connect().attempt(false).err().unwrap().give_up();
    println!("gave up: {} log {:?}", failed, failed.log);
    println!("sizes: Closed={} Open={} Conn<Closed>={}", std::mem::size_of::<Closed>(), std::mem::size_of::<Open>(), std::mem::size_of::<Conn<Closed>>() == std::mem::size_of::<Conn<Closed>>());
    let req = ReqBuilder::new().header("a", "1").method("POST").url("/api").header("b", "2").build();
    println!("{}", req);
    println!("{}", ReqBuilder::new().url("/x").method("GET").build());
}
