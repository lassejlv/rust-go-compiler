// `?` inside closures and async blocks whose error type is only decided after the closure
// (by a later annotation reached through method calls/fields), converting with From.
use std::error::Error;
use std::fmt;
use std::future::Future;
use std::pin::pin;
use std::task::{Context, Poll, Waker};

type BoxErr = Box<dyn Error + Send + Sync>;

#[derive(Debug)]
struct AppErr(String);
impl fmt::Display for AppErr {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "app: {}", self.0)
    }
}
impl From<std::num::ParseIntError> for AppErr {
    fn from(e: std::num::ParseIntError) -> Self {
        AppErr(e.to_string())
    }
}
impl From<std::str::Utf8Error> for AppErr {
    fn from(e: std::str::Utf8Error) -> Self {
        AppErr(format!("utf8 {}", e.valid_up_to()))
    }
}

struct Wrap<T>(T);
impl<T> Wrap<T> {
    fn get(self) -> T {
        self.0
    }
}
fn wrap<T, F: FnOnce() -> T>(f: F) -> Wrap<T> {
    Wrap(f())
}

fn block_on<F: Future>(f: F) -> F::Output {
    let mut cx = Context::from_waker(Waker::noop());
    let mut f = pin!(f);
    loop {
        if let Poll::Ready(v) = f.as_mut().poll(&mut cx) {
            return v;
        }
    }
}

fn main() {
    let r: Result<u32, BoxErr> = std::thread::spawn(|| {
        let n: u32 = "21".parse()?;
        Ok(n * 2)
    })
    .join()
    .unwrap();
    println!("{:?}", r.map_err(|e| e.to_string()));

    let r: Result<u32, AppErr> = wrap(|| {
        let a: u32 = "4".parse()?;
        let s = std::str::from_utf8(&[0x61, 0xff])?;
        Ok(a + s.len() as u32)
    })
    .get();
    println!("{:?}", r.map_err(|e| e.to_string()));

    let r: Result<i64, AppErr> = wrap(|| Ok("x9".parse::<i64>()?)).0;
    println!("{:?}", r.map_err(|e| e.to_string()));

    let c = || -> Result<u8, AppErr> { Ok("300".parse::<u8>()?) };
    println!("{}", c().unwrap_err());

    let fut = async {
        let n: u16 = "70000".parse()?;
        Ok(n)
    };
    let r: Result<u16, BoxErr> = block_on(fut);
    println!("{:?}", r.map_err(|e| e.to_string()));

    let results: Vec<Result<u8, AppErr>> = ["1", "2", "zz"]
        .iter()
        .map(|s| {
            let v: u8 = s.parse()?;
            Ok(v * 10)
        })
        .collect();
    for r in results {
        println!("{:?}", r.map_err(|e| e.to_string()));
    }
}
