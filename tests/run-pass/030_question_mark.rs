use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError {
    Parse(ParseIntError),
    Negative(i64),
    TooLarge { value: i64, max: i64 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Parse(e) => write!(f, "parse error: {}", e),
            AppError::Negative(v) => write!(f, "negative value: {}", v),
            AppError::TooLarge { value, max } => write!(f, "{} exceeds {}", value, max),
        }
    }
}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> Self {
        AppError::Parse(e)
    }
}

fn parse_checked(s: &str) -> Result<i64, AppError> {
    let v: i64 = s.trim().parse()?;
    if v < 0 {
        return Err(AppError::Negative(v));
    }
    if v > 1000 {
        return Err(AppError::TooLarge { value: v, max: 1000 });
    }
    Ok(v)
}

fn sum_all(items: &[&str]) -> Result<i64, AppError> {
    let mut total = 0;
    for item in items {
        total += parse_checked(item)?;
    }
    Ok(total)
}

fn first_char_upper(s: &str) -> Option<char> {
    let c = s.chars().next()?;
    Some(c.to_ascii_uppercase())
}

fn second_word_len(s: &str) -> Option<usize> {
    let w = s.split_whitespace().nth(1)?;
    Some(w.len())
}

fn parse_pair(s: &str) -> Result<(i32, i32), String> {
    let mut parts = s.split(',');
    let a = parts.next().ok_or("missing first")?.trim().parse::<i32>().map_err(|e| e.to_string())?;
    let b = parts.next().ok_or("missing second")?.trim().parse::<i32>().map_err(|e| e.to_string())?;
    Ok((a, b))
}

fn main() {
    for s in ["42", " 7 ", "-5", "5000", "abc", ""] {
        match parse_checked(s) {
            Ok(v) => println!("{:?} -> ok {}", s, v),
            Err(e) => println!("{:?} -> {}", s, e),
        }
    }
    println!("sum = {:?}", sum_all(&["1", "2", "3"]).ok());
    match sum_all(&["1", "-2", "3"]) {
        Ok(v) => println!("sum = {}", v),
        Err(e) => println!("sum failed: {}", e),
    }
    println!("{:?} {:?}", first_char_upper("rust"), first_char_upper(""));
    println!("{:?} {:?}", second_word_len("hello big world"), second_word_len("single"));
    for s in ["3, 4", "10,x", "5", ""] {
        println!("parse_pair({:?}) = {:?}", s, parse_pair(s));
    }
}
