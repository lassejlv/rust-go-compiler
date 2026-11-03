use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError<E> {
    Parse(ParseIntError),
    Domain(E),
    Missing(&'static str),
}

impl<E: fmt::Display> fmt::Display for AppError<E> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "parse error: {}", e),
            AppError::Domain(e) => write!(f, "domain error: {}", e),
            AppError::Missing(k) => write!(f, "missing {}", k),
        }
    }
}

impl<E> From<ParseIntError> for AppError<E> {
    fn from(e: ParseIntError) -> Self { AppError::Parse(e) }
}

#[derive(Debug)]
struct RangeErr { val: i64, max: i64 }
impl fmt::Display for RangeErr {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { write!(f, "{} exceeds {}", self.val, self.max) }
}
impl From<RangeErr> for AppError<RangeErr> {
    fn from(e: RangeErr) -> Self { AppError::Domain(e) }
}

fn check(v: i64, max: i64) -> Result<i64, RangeErr> {
    if v > max { Err(RangeErr { val: v, max }) } else { Ok(v) }
}

fn parse_kv(s: &str) -> Result<(String, i64), AppError<RangeErr>> {
    let mut parts = s.splitn(2, '=');
    let k = parts.next().filter(|k| !k.is_empty()).ok_or(AppError::Missing("key"))?;
    let v = parts.next().ok_or(AppError::Missing("value"))?;
    let n: i64 = v.trim().parse()?;
    let n = check(n, 100)?;
    Ok((k.to_string(), n))
}

fn sum_all<E>(items: &[&str], f: impl Fn(&str) -> Result<i64, AppError<E>>) -> Result<i64, AppError<E>> {
    let mut t = 0;
    for i in items { t += f(i)?; }
    Ok(t)
}

#[derive(Debug)]
struct Ctx<E> { ctx: String, source: E }
trait Context<T, E> { fn context(self, c: &str) -> Result<T, Ctx<E>>; }
impl<T, E> Context<T, E> for Result<T, E> {
    fn context(self, c: &str) -> Result<T, Ctx<E>> { self.map_err(|e| Ctx { ctx: c.to_string(), source: e }) }
}

fn load(s: &str) -> Result<u8, Ctx<ParseIntError>> {
    let v: u8 = s.parse().context("loading byte")?;
    Ok(v.wrapping_mul(2))
}

fn boxed(s: &str) -> Result<i64, Box<dyn std::error::Error>> {
    let n: i64 = s.parse()?;
    if n < 0 { return Err(format!("negative: {}", n).into()); }
    Ok(n * 2)
}

fn main() {
    for s in ["a=5", "b= 42", "=3", "c", "d=xx", "e=500", "f=-7"] {
        match parse_kv(s) {
            Ok((k, v)) => println!("ok {} {}", k, v),
            Err(e) => println!("err {}", e),
        }
    }
    let r = sum_all(&["1", "2", "3"], |s| Ok::<i64, AppError<String>>(s.parse::<i64>()?));
    println!("{:?}", r.map_err(|e| e.to_string()));
    let r2 = sum_all(&["1", "zz"], |s| Ok::<i64, AppError<String>>(s.parse::<i64>()?));
    println!("{}", r2.unwrap_err());
    let r3 = sum_all(&["1", "9"], |s| {
        let n: i64 = s.parse()?;
        if n > 5 { Err(AppError::Domain(format!("{} too big", n))) } else { Ok(n) }
    });
    println!("{}", r3.unwrap_err());
    println!("{:?}", load("100"));
    match load("300") { Err(Ctx { ctx, source }) => println!("{}: {}", ctx, source), Ok(v) => println!("{}", v) }
    for s in ["21", "-1", "q"] {
        match boxed(s) { Ok(v) => println!("boxed ok {}", v), Err(e) => println!("boxed err {}", e) }
    }
}
