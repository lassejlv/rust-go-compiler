// An anyhow-like error type: boxed dyn Error + Send + Sync, context chains, downcasting, bail!/ensure!.
use std::error::Error as StdError;
use std::fmt;

type BoxError = Box<dyn StdError + Send + Sync + 'static>;

struct Error {
    inner: BoxError,
}

#[derive(Debug)]
struct ContextError {
    msg: String,
    source: BoxError,
}

impl fmt::Display for ContextError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.msg)
    }
}
impl StdError for ContextError {
    fn source(&self) -> Option<&(dyn StdError + 'static)> {
        Some(self.source.as_ref())
    }
}

#[derive(Debug)]
struct MsgError(String);
impl fmt::Display for MsgError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        f.write_str(&self.0)
    }
}
impl StdError for MsgError {}

impl Error {
    fn msg(m: impl fmt::Display) -> Self {
        Error { inner: Box::new(MsgError(m.to_string())) }
    }
    fn context(self, msg: impl fmt::Display) -> Self {
        Error { inner: Box::new(ContextError { msg: msg.to_string(), source: self.inner }) }
    }
    fn chain(&self) -> Vec<String> {
        let mut v = vec![];
        let mut cur: Option<&(dyn StdError + 'static)> = Some(self.inner.as_ref());
        while let Some(e) = cur {
            v.push(e.to_string());
            cur = e.source();
        }
        v
    }
    fn root_cause(&self) -> &(dyn StdError + 'static) {
        let mut cur: &(dyn StdError + 'static) = self.inner.as_ref();
        while let Some(n) = cur.source() {
            cur = n;
        }
        cur
    }
    fn downcast_ref<E: StdError + 'static>(&self) -> Option<&E> {
        let mut cur: Option<&(dyn StdError + 'static)> = Some(self.inner.as_ref());
        while let Some(e) = cur {
            if let Some(x) = e.downcast_ref::<E>() {
                return Some(x);
            }
            cur = e.source();
        }
        None
    }
}

impl<E: StdError + Send + Sync + 'static> From<E> for Error {
    fn from(e: E) -> Self {
        Error { inner: Box::new(e) }
    }
}

impl fmt::Debug for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let chain = self.chain();
        write!(f, "{}", chain[0])?;
        if chain.len() > 1 {
            write!(f, "\n\nCaused by:")?;
            for (i, c) in chain[1..].iter().enumerate() {
                write!(f, "\n    {}: {}", i, c)?;
            }
        }
        Ok(())
    }
}

impl fmt::Display for Error {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if f.alternate() {
            write!(f, "{}", self.chain().join(": "))
        } else {
            write!(f, "{}", self.inner)
        }
    }
}

type Result<T, E = Error> = std::result::Result<T, E>;

trait Context<T> {
    fn context<C: fmt::Display>(self, c: C) -> Result<T>;
    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T>;
}

impl<T, E: Into<Error>> Context<T> for std::result::Result<T, E> {
    fn context<C: fmt::Display>(self, c: C) -> Result<T> {
        self.map_err(|e| e.into().context(c))
    }
    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T> {
        self.map_err(|e| e.into().context(f()))
    }
}

impl<T> Context<T> for Option<T> {
    fn context<C: fmt::Display>(self, c: C) -> Result<T> {
        self.ok_or_else(|| Error::msg(c))
    }
    fn with_context<C: fmt::Display, F: FnOnce() -> C>(self, f: F) -> Result<T> {
        self.ok_or_else(|| Error::msg(f()))
    }
}

macro_rules! bail {
    ($($arg:tt)*) => { return Err(Error::msg(format!($($arg)*))) };
}
macro_rules! ensure {
    ($cond:expr, $($arg:tt)*) => { if !$cond { bail!($($arg)*); } };
}

#[derive(Debug)]
struct NotFound {
    id: u32,
}
impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "item {} not found", self.id)
    }
}
impl StdError for NotFound {}

fn lookup(id: u32) -> Result<&'static str, NotFound> {
    match id {
        1 => Ok("10"),
        2 => Ok("2x"),
        _ => Err(NotFound { id }),
    }
}

fn parse_item(id: u32) -> Result<i32> {
    let s = lookup(id).with_context(|| format!("looking up id {}", id))?;
    let n: i32 = s.parse().context("parsing item text")?;
    Ok(n)
}

fn process(ids: &[u32]) -> Result<i32> {
    ensure!(!ids.is_empty(), "no ids given");
    let mut total = 0;
    for &id in ids {
        if id == 99 {
            bail!("id {} is reserved", id);
        }
        total += parse_item(id).with_context(|| format!("processing batch {:?}", ids))?;
    }
    let first = ids.first().context("unreachable")?;
    Ok(total * *first as i32)
}

fn main() {
    let cases: [&[u32]; 5] = [&[1, 1], &[1, 2], &[3], &[], &[1, 99]];
    for c in cases {
        match process(c) {
            Ok(v) => println!("{:?} => ok {}", c, v),
            Err(e) => {
                println!("{:?} => error: {}", c, e);
                println!("  alt: {:#}", e);
                println!("  root: {}", e.root_cause());
                if let Some(nf) = e.downcast_ref::<NotFound>() {
                    println!("  downcast NotFound id={}", nf.id);
                }
                if let Some(pe) = e.downcast_ref::<std::num::ParseIntError>() {
                    println!("  downcast ParseIntError: {:?}", pe.kind());
                }
                println!("  debug:\n{:?}", e);
            }
        }
    }
    let opt: Option<u8> = None;
    let r: Result<u8> = opt.context("value was missing");
    println!("{}", r.unwrap_err());
}
