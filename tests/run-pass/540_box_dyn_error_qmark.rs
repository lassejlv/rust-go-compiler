// Box<dyn Error + Send + Sync>, ? with From conversions across layers, custom From impls
use std::error::Error;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};

type BoxResult<T> = Result<T, Box<dyn Error + Send + Sync>>;

#[derive(Debug)]
enum CalcError {
    Int(ParseIntError),
    Float(ParseFloatError),
    DivZero,
    Custom(String),
}
impl fmt::Display for CalcError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CalcError::Int(e) => write!(f, "int: {}", e),
            CalcError::Float(e) => write!(f, "float: {}", e),
            CalcError::DivZero => write!(f, "division by zero"),
            CalcError::Custom(s) => write!(f, "custom: {}", s),
        }
    }
}
impl Error for CalcError {}
impl From<ParseIntError> for CalcError {
    fn from(e: ParseIntError) -> Self {
        CalcError::Int(e)
    }
}
impl From<ParseFloatError> for CalcError {
    fn from(e: ParseFloatError) -> Self {
        CalcError::Float(e)
    }
}
impl From<&str> for CalcError {
    fn from(s: &str) -> Self {
        CalcError::Custom(s.to_string())
    }
}

fn div(a: &str, b: &str) -> Result<i64, CalcError> {
    let a: i64 = a.parse()?;
    let b: i64 = b.parse()?;
    if b == 0 {
        return Err(CalcError::DivZero);
    }
    Ok(a / b)
}

fn scale(x: &str, f: &str) -> Result<f64, CalcError> {
    let x = x.parse::<i32>()? as f64;
    let f: f64 = f.parse()?;
    if f < 0.0 {
        Err("negative factor")?;
    }
    Ok(x * f)
}

fn boxed(a: &str, b: &str) -> BoxResult<String> {
    let q = div(a, b)?;
    let s = scale(&q.to_string(), "1.5")?;
    if s > 100.0 {
        return Err(format!("too big: {}", s).into());
    }
    let n: u8 = "300".parse().or_else(|_| "30".parse())?;
    Ok(format!("{} {}", s, n))
}

fn opt_chain(v: &[&str]) -> Option<i32> {
    let first = v.first()?;
    let n: i32 = first.parse().ok()?;
    let second = v.get(1)?.parse::<i32>().ok()?;
    Some(n.checked_mul(second)?)
}

fn thread_err() -> BoxResult<i32> {
    let h = std::thread::spawn(|| -> BoxResult<i32> {
        let v: i32 = "21".parse()?;
        Ok(v * 2)
    });
    let r = h.join().map_err(|_| "thread panicked")??;
    Ok(r)
}

fn main() {
    println!("{:?}", div("10", "3"));
    for (a, b) in [("10", "0"), ("x", "1"), ("1", "")] {
        match div(a, b) {
            Ok(v) => println!("ok {}", v),
            Err(e) => println!("err {} / {:?}", e, e),
        }
    }
    for (x, f) in [("4", "2.5"), ("4", "abc"), ("4", "-1"), ("q", "1")] {
        println!("{:?}", scale(x, f).map_err(|e| e.to_string()));
    }
    for (a, b) in [("100", "2"), ("100", "0"), ("1000", "1"), ("z", "1")] {
        match boxed(a, b) {
            Ok(s) => println!("boxed ok {}", s),
            Err(e) => {
                let kind = if e.is::<CalcError>() { "calc" } else { "other" };
                println!("boxed err [{}] {}", kind, e);
            }
        }
    }
    println!("{:?} {:?} {:?} {:?}", opt_chain(&["6", "7"]), opt_chain(&["6"]), opt_chain(&[]), opt_chain(&["99999", "99999"]));
    println!("{:?}", thread_err().map_err(|e| e.to_string()));
    let e: Box<dyn Error + Send + Sync> = Box::new(CalcError::DivZero);
    let e2: Box<dyn Error> = e;
    println!("{}", e2);
    match e2.downcast::<CalcError>() {
        Ok(c) => println!("downcast ok: {:?}", c),
        Err(_) => println!("downcast failed"),
    }
    fn needs_static(e: Box<dyn Error + Send + Sync + 'static>) -> String {
        format!("static {}", e)
    }
    println!("{}", needs_static("x".into()));
    let results: Vec<BoxResult<i32>> = vec![Ok(1), Err("boom".into()), Ok(3)];
    let collected: Result<Vec<i32>, _> = results.into_iter().collect();
    println!("{:?}", collected.map_err(|e| e.to_string()));
}
