// Box<dyn Error + Send + Sync> chains: errors from strings, `?` converting concrete errors into Box,
// downcasting (downcast_ref, downcast::<T>()), custom context wrapper with source(), Error::description
// avoided, iterating sources, and errors crossing a thread boundary.
use std::error::Error;
use std::fmt;

type BoxErr = Box<dyn Error + Send + Sync + 'static>;

#[derive(Debug)]
struct Context {
    msg: String,
    inner: BoxErr,
}
impl fmt::Display for Context {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.msg)
    }
}
impl Error for Context {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(self.inner.as_ref())
    }
}

trait WithContext<T> {
    fn context(self, msg: impl Into<String>) -> Result<T, BoxErr>;
}
impl<T, E: Into<BoxErr>> WithContext<T> for Result<T, E> {
    fn context(self, msg: impl Into<String>) -> Result<T, BoxErr> {
        self.map_err(|e| Box::new(Context { msg: msg.into(), inner: e.into() }) as BoxErr)
    }
}

#[derive(Debug, PartialEq)]
struct NotFound(String);
impl fmt::Display for NotFound {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{} not found", self.0)
    }
}
impl Error for NotFound {}

fn lookup(key: &str) -> Result<i32, BoxErr> {
    match key {
        "a" => Ok(1),
        "b" => Ok(2),
        "num" => Ok("12x".parse::<i32>()?),
        "empty" => Err("empty key given".into()),
        k => Err(Box::new(NotFound(k.to_string()))),
    }
}

fn compute(keys: &[&str]) -> Result<i32, BoxErr> {
    let mut sum = 0;
    for k in keys {
        sum += lookup(k).context(format!("looking up {:?}", k))?;
    }
    if sum > 2 {
        return Err(format!("sum {} too large", sum).into());
    }
    Ok(sum)
}

fn report(e: &(dyn Error + 'static)) -> String {
    let mut s = format!("{}", e);
    let mut depth = 0;
    let mut cur = e.source();
    while let Some(c) = cur {
        depth += 1;
        s.push_str(&format!("\n  {}caused by: {}", "  ".repeat(depth - 1), c));
        cur = c.source();
    }
    s
}

fn classify(e: &BoxErr) -> &'static str {
    let root: &(dyn Error + 'static) = {
        let mut cur: &(dyn Error + 'static) = e.as_ref();
        while let Some(s) = cur.source() {
            cur = s;
        }
        cur
    };
    if root.downcast_ref::<NotFound>().is_some() {
        "not-found"
    } else if root.is::<std::num::ParseIntError>() {
        "parse"
    } else {
        "other"
    }
}

fn main() {
    for keys in [&["a"][..], &["a", "b"], &["a", "zzz"], &["num"], &["empty"], &[]] {
        match compute(keys) {
            Ok(v) => println!("{:?} => {}", keys, v),
            Err(e) => println!("{:?} => [{}] {}", keys, classify(&e), report(e.as_ref())),
        }
    }

    let e: BoxErr = Box::new(NotFound("cfg".into()));
    match e.downcast::<NotFound>() {
        Ok(nf) => println!("downcast ok: {:?}", *nf),
        Err(_) => println!("downcast failed"),
    }
    let e2: BoxErr = "plain message".into();
    println!("{:?}", e2.to_string());
    println!("{}", e2.downcast::<NotFound>().is_err());

    let from_string: Box<dyn Error> = String::from("owned msg").into();
    println!("{} / {:?}", from_string, from_string.source().is_none());

    let handle = std::thread::spawn(|| -> Result<u32, BoxErr> {
        let v: u32 = "77".parse()?;
        let w: u32 = "-1".parse().context("parsing w")?;
        Ok(v + w)
    });
    match handle.join().unwrap() {
        Ok(v) => println!("thread ok {}", v),
        Err(e) => println!("thread err: {}", report(e.as_ref())),
    }

    let results: Vec<Result<i32, BoxErr>> = vec![lookup("a"), lookup("x"), lookup("b")];
    let (oks, errs): (Vec<_>, Vec<_>) = results.into_iter().partition(|r| r.is_ok());
    println!("{} ok, {} err", oks.len(), errs.len());
    let errs: Vec<String> = errs.into_iter().map(|r| r.unwrap_err().to_string()).collect();
    println!("{:?}", errs);
    let io = std::io::Error::new(std::io::ErrorKind::Other, NotFound("disk".into()));
    println!("{} | inner={:?}", io, io.get_ref().map(|e| e.to_string()));
    let wrapped: Result<(), BoxErr> = Err(io).context("saving file");
    println!("{}", report(wrapped.unwrap_err().as_ref()));
}
