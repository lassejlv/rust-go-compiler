use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct NegativeError(i32);

impl fmt::Display for NegativeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "negative value: {}", self.0)
    }
}

impl Error for NegativeError {}

fn check(v: i32) -> Result<i32, NegativeError> {
    if v < 0 {
        Err(NegativeError(v))
    } else {
        Ok(v * 2)
    }
}

fn parse_and_check(s: &str) -> Result<i32, Box<dyn Error>> {
    let n: i32 = s.parse()?;
    let d = check(n)?;
    Ok(d)
}

fn main() -> Result<(), Box<dyn Error>> {
    let a = parse_and_check("21")?;
    println!("a = {}", a);
    let b = parse_and_check("-4");
    match &b {
        Ok(v) => println!("b = {}", v),
        Err(e) => println!("b failed: {}", e),
    }
    let c = parse_and_check("nope").unwrap_or(-1);
    println!("c = {}", c);
    let total: i32 = ["1", "2", "3"].iter().map(|s| parse_and_check(s)).collect::<Result<Vec<_>, _>>()?.iter().sum();
    println!("total = {}", total);
    let s = String::from_utf8(vec![104, 105])?;
    println!("s = {}", s);
    println!("done");
    Ok(())
}
