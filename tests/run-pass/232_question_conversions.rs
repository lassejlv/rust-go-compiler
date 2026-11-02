// `?` across many conversions: Option, Result with From, Box<dyn Error>, custom Try-like flows.
use std::collections::BTreeMap;
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;
use std::str::Utf8Error;

#[derive(Debug, PartialEq)]
enum LowError {
    Empty,
    TooLong(usize),
}
impl fmt::Display for LowError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            LowError::Empty => write!(f, "empty input"),
            LowError::TooLong(n) => write!(f, "too long ({} bytes)", n),
        }
    }
}
impl Error for LowError {}

#[derive(Debug)]
enum AppError {
    Low(LowError),
    Parse(ParseIntError),
    Utf8(Utf8Error),
    Other(String),
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Low(e) => write!(f, "low: {}", e),
            AppError::Parse(e) => write!(f, "parse: {}", e),
            AppError::Utf8(e) => write!(f, "utf8: {}", e),
            AppError::Other(s) => write!(f, "other: {}", s),
        }
    }
}
impl Error for AppError {}
impl From<LowError> for AppError {
    fn from(e: LowError) -> Self {
        AppError::Low(e)
    }
}
impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self {
        AppError::Parse(e)
    }
}
impl From<Utf8Error> for AppError {
    fn from(e: Utf8Error) -> Self {
        AppError::Utf8(e)
    }
}
impl From<String> for AppError {
    fn from(s: String) -> Self {
        AppError::Other(s)
    }
}

fn check(b: &[u8]) -> Result<&[u8], LowError> {
    if b.is_empty() {
        Err(LowError::Empty)
    } else if b.len() > 8 {
        Err(LowError::TooLong(b.len()))
    } else {
        Ok(b)
    }
}

fn decode(b: &[u8]) -> Result<i64, AppError> {
    let b = check(b)?;
    let s = std::str::from_utf8(b)?;
    let n: i64 = s.trim().parse()?;
    if n < 0 {
        return Err(format!("negative: {}", n).into());
    }
    Ok(n)
}

fn decode_boxed(b: &[u8]) -> Result<i64, Box<dyn Error>> {
    let n = decode(b)?;
    if n == 13 {
        Err("unlucky")?;
    }
    Ok(n * 2)
}

fn decode_send(b: &[u8]) -> Result<i64, Box<dyn Error + Send + Sync>> {
    let s = std::str::from_utf8(b)?;
    Ok(s.len() as i64 + check(b)?.len() as i64)
}

// ? on Option inside functions returning Option
fn middle_word_len(s: &str) -> Option<usize> {
    let words: Vec<&str> = s.split_whitespace().collect();
    let mid = words.get(words.len() / 2)?;
    let first = mid.chars().next()?;
    if first.is_uppercase() {
        Some(mid.len() * 10)
    } else {
        Some(mid.len())
    }
}

fn lookup_chain(m: &BTreeMap<&str, &str>, start: &str, hops: usize) -> Option<String> {
    let mut cur = start;
    for _ in 0..hops {
        cur = m.get(cur)?;
    }
    Some(cur.to_uppercase())
}

// ? inside closures returning Result, with try-collect
fn parse_all(items: &[&str]) -> Result<Vec<u8>, ParseIntError> {
    items.iter().map(|s| s.parse::<u8>()).collect()
}

fn sum_valid(items: &[&str]) -> Result<u32, String> {
    let parsed = items
        .iter()
        .map(|s| -> Result<u32, String> {
            let v: u32 = s.parse().map_err(|e: ParseIntError| format!("{:?}: {}", s, e))?;
            Ok(v)
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(parsed.iter().sum())
}

fn opt_sum(items: &[Option<i32>]) -> Option<i32> {
    items.iter().copied().sum()
}

fn main() {
    for inp in [&b"42"[..], b" 7 ", b"", b"123456789", b"-5", b"abc", &[0xff, 0x31], b"13"] {
        let d = decode(inp);
        let b = decode_boxed(inp);
        let s = decode_send(inp);
        println!(
            "{:<14} decode={:<34} boxed={:<26} send={}",
            format!("{:?}", String::from_utf8_lossy(inp)),
            match &d {
                Ok(v) => format!("Ok({})", v),
                Err(e) => format!("Err({})", e),
            },
            match &b {
                Ok(v) => format!("Ok({})", v),
                Err(e) => format!("Err({})", e),
            },
            match &s {
                Ok(v) => format!("Ok({})", v),
                Err(e) => format!("Err({})", e),
            }
        );
    }
    println!("{:?}", middle_word_len("the Quick fox"));
    println!("{:?}", middle_word_len("a b c d"));
    println!("{:?}", middle_word_len(""));
    let m: BTreeMap<&str, &str> = [("a", "b"), ("b", "c"), ("c", "d")].into_iter().collect();
    println!("{:?} {:?} {:?}", lookup_chain(&m, "a", 2), lookup_chain(&m, "a", 3), lookup_chain(&m, "a", 4));
    println!("{:?}", parse_all(&["1", "2", "255"]));
    println!("{:?}", parse_all(&["1", "256"]).map_err(|e| e.to_string()));
    println!("{:?}", sum_valid(&["10", "20", "30"]));
    println!("{:?}", sum_valid(&["10", "x", "30"]));
    println!("{:?} {:?}", opt_sum(&[Some(1), Some(2)]), opt_sum(&[Some(1), None]));
    let e: AppError = LowError::TooLong(99).into();
    println!("{} | source is none: {}", e, e.source().is_none());
}
