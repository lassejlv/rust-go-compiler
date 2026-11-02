// Custom error enum with From conversions for ? across layers (ParseIntError, ParseFloatError,
// Utf8Error, custom sub-errors), Display/Error impls with source(), error mapping and context
// wrapping, and Result chains through several functions.
use std::error::Error;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum FieldError {
    Missing(&'static str),
    Range { field: &'static str, value: i64, min: i64, max: i64 },
}

impl fmt::Display for FieldError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            FieldError::Missing(name) => write!(f, "missing field `{}`", name),
            FieldError::Range { field, value, min, max } => write!(f, "{} = {} out of range [{}, {}]", field, value, min, max),
        }
    }
}
impl Error for FieldError {}

#[derive(Debug)]
enum AppError {
    Int(ParseIntError),
    Float(ParseFloatError),
    Utf8(std::str::Utf8Error),
    Field(FieldError),
    Line { line: usize, source: Box<AppError> },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Int(_) => write!(f, "invalid integer"),
            AppError::Float(_) => write!(f, "invalid float"),
            AppError::Utf8(_) => write!(f, "invalid utf-8"),
            AppError::Field(e) => write!(f, "field error: {}", e),
            AppError::Line { line, .. } => write!(f, "error on line {}", line),
        }
    }
}

impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            AppError::Int(e) => Some(e),
            AppError::Float(e) => Some(e),
            AppError::Utf8(e) => Some(e),
            AppError::Field(e) => Some(e),
            AppError::Line { source, .. } => Some(source.as_ref()),
        }
    }
}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self {
        AppError::Int(e)
    }
}
impl From<ParseFloatError> for AppError {
    fn from(e: ParseFloatError) -> Self {
        AppError::Float(e)
    }
}
impl From<std::str::Utf8Error> for AppError {
    fn from(e: std::str::Utf8Error) -> Self {
        AppError::Utf8(e)
    }
}
impl From<FieldError> for AppError {
    fn from(e: FieldError) -> Self {
        AppError::Field(e)
    }
}

#[derive(Debug)]
struct Item {
    id: u32,
    qty: i64,
    price: f64,
}

fn check_range(field: &'static str, value: i64, min: i64, max: i64) -> Result<i64, FieldError> {
    if value < min || value > max {
        Err(FieldError::Range { field, value, min, max })
    } else {
        Ok(value)
    }
}

fn parse_item(bytes: &[u8]) -> Result<Item, AppError> {
    let line = std::str::from_utf8(bytes)?;
    let mut parts = line.split(',').map(str::trim);
    let id = parts.next().filter(|s| !s.is_empty()).ok_or(FieldError::Missing("id"))?.parse::<u32>()?;
    let qty_raw = parts.next().ok_or(FieldError::Missing("qty"))?.parse::<i64>()?;
    let qty = check_range("qty", qty_raw, 1, 100)?;
    let price = parts.next().ok_or(FieldError::Missing("price"))?.parse::<f64>()?;
    Ok(Item { id, qty, price })
}

fn parse_all(input: &[&[u8]]) -> Result<Vec<Item>, AppError> {
    input
        .iter()
        .enumerate()
        .map(|(i, l)| parse_item(l).map_err(|e| AppError::Line { line: i + 1, source: Box::new(e) }))
        .collect()
}

fn chain(e: &dyn Error) -> String {
    let mut parts = vec![e.to_string()];
    let mut cur = e.source();
    while let Some(s) = cur {
        parts.push(s.to_string());
        cur = s.source();
    }
    parts.join(" <- ")
}

fn total(input: &[&[u8]]) -> Result<f64, Box<dyn Error>> {
    let items = parse_all(input)?;
    Ok(items.iter().map(|i| i.qty as f64 * i.price).sum())
}

fn main() {
    let lines: Vec<&[u8]> = vec![b"1, 2, 3.5", b"x,1,1", b"2,0,1", b"3,5,abc", b",1,1", b"4,7", b"\xff,1,1", b"5,100,0.25", b"99999999999,1,1"];
    for l in &lines {
        match parse_item(l) {
            Ok(item) => println!("ok {:?}", item),
            Err(e) => println!("err: {} | {:?}", chain(&e), e.source().map(|s| s.to_string())),
        }
    }
    let good: Vec<&[u8]> = vec![b"1,2,3.5", b"5,100,0.25"];
    println!("{:?}", total(&good));
    let bad: Vec<&[u8]> = vec![b"1,2,3.5", b"2,500,1"];
    match total(&bad) {
        Ok(t) => println!("total {}", t),
        Err(e) => println!("total failed: {}", chain(e.as_ref())),
    }
    let r: Result<u8, AppError> = "300".parse::<u8>().map_err(AppError::from);
    println!("{}", matches!(r, Err(AppError::Int(_))));
    let o: Option<i64> = parse_item(b"1,2,3").ok().map(|i| i.qty);
    println!("{:?}", o);
}
