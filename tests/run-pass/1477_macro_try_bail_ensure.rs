// error-handling macros: try-like, bail!, ensure!, context-chaining, let-else in macros
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError { Parse(ParseIntError), Msg(String), Range { v: i64, max: i64 } }
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "parse error: {}", e),
            AppError::Msg(m) => write!(f, "{}", m),
            AppError::Range { v, max } => write!(f, "{} out of range (max {})", v, max),
        }
    }
}
impl From<ParseIntError> for AppError { fn from(e: ParseIntError) -> Self { AppError::Parse(e) } }

macro_rules! tri {
    ($e:expr) => { match $e { Ok(v) => v, Err(e) => return Err(::std::convert::From::from(e)) } };
}
macro_rules! bail {
    ($($arg:tt)+) => { return Err(AppError::Msg(format!($($arg)+))) };
}
macro_rules! ensure {
    ($cond:expr, $($arg:tt)+) => { if !$cond { bail!($($arg)+); } };
}
macro_rules! get_or {
    ($opt:expr, $default:expr) => {{ let Some(v) = $opt else { return Ok($default) }; v }};
}

fn parse_bounded(s: &str, max: i64) -> Result<i64, AppError> {
    let v: i64 = tri!(s.trim().parse::<i64>());
    ensure!(v >= 0, "negative value: {}", v);
    if v > max { return Err(AppError::Range { v, max }); }
    Ok(v)
}

fn first_doubled(items: &[&str]) -> Result<i64, AppError> {
    let first = get_or!(items.first(), -1);
    let v = parse_bounded(first, 100)?;
    if v == 13 { bail!("unlucky {}", v); }
    Ok(v * 2)
}

fn main() {
    for input in [&["7"][..], &["x"], &["-3"], &["500"], &[], &["13"], &[" 42 "]] {
        match first_doubled(input) {
            Ok(v) => println!("ok {}", v),
            Err(e) => println!("err {} / {:?}", e, e),
        }
    }
}
