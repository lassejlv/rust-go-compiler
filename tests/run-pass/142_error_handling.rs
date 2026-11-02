use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum ConfigError {
    Missing(String),
    BadNumber { key: String, source: ParseIntError },
    OutOfRange(i64),
}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Missing(k) => write!(f, "missing key '{}'", k),
            ConfigError::BadNumber { key, source } => write!(f, "bad number for {}: {}", key, source),
            ConfigError::OutOfRange(v) => write!(f, "value {} out of range", v),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::BadNumber { source, .. } => Some(source),
            _ => None,
        }
    }
}

#[derive(Debug)]
struct AppError {
    context: String,
    inner: ConfigError,
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}: {}", self.context, self.inner)
    }
}
impl Error for AppError {}

impl From<ConfigError> for AppError {
    fn from(e: ConfigError) -> Self {
        AppError { context: "config".to_string(), inner: e }
    }
}

fn lookup<'a>(pairs: &'a [(&'a str, &'a str)], key: &str) -> Result<&'a str, ConfigError> {
    pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).ok_or_else(|| ConfigError::Missing(key.to_string()))
}

fn get_num(pairs: &[(&str, &str)], key: &str) -> Result<i64, ConfigError> {
    let raw = lookup(pairs, key)?;
    let n: i64 = raw.trim().parse().map_err(|e| ConfigError::BadNumber { key: key.to_string(), source: e })?;
    if !(0..=1000).contains(&n) {
        return Err(ConfigError::OutOfRange(n));
    }
    Ok(n)
}

fn app(pairs: &[(&str, &str)]) -> Result<i64, AppError> {
    let w = get_num(pairs, "width")?;
    let h = get_num(pairs, "height")?;
    Ok(w * h)
}

fn mixed(input: &str) -> Result<f64, Box<dyn Error>> {
    let mut parts = input.split(',');
    let a: i32 = parts.next().ok_or("no first part")?.parse()?;
    let b: f64 = parts.next().ok_or_else(|| format!("no second part in {:?}", input))?.parse()?;
    if b == 0.0 {
        return Err(Box::new(ConfigError::OutOfRange(0)));
    }
    Ok(a as f64 / b)
}

fn chain(e: &dyn Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    while let Some(c) = cur {
        s.push_str(" <- ");
        s.push_str(&c.to_string());
        cur = c.source();
    }
    s
}

fn main() {
    let good = [("width", "20"), ("height", " 30 ")];
    let missing = [("width", "20")];
    let bad = [("width", "2x0"), ("height", "1")];
    let range = [("width", "5"), ("height", "5000")];
    for cfg in [&good[..], &missing[..], &bad[..], &range[..]] {
        match app(cfg) {
            Ok(v) => println!("ok {}", v),
            Err(e) => println!("err {} | chain: {}", e, chain(&e.inner)),
        }
    }
    for input in ["10,4", "7", "x,1", "3,abc", "5,0", ""] {
        match mixed(input) {
            Ok(v) => println!("{:?} -> {}", input, v),
            Err(e) => {
                let is_cfg = e.downcast_ref::<ConfigError>().is_some();
                let is_parse = e.is::<ParseIntError>();
                println!("{:?} -> error: {} (cfg={}, parse_int={})", input, e, is_cfg, is_parse);
            }
        }
    }
    let r: Result<Vec<i32>, _> = "1 2 3".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("{:?}", r);
    let r: Result<Vec<i32>, _> = "1 two 3".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("{:?}", r.map_err(|e| e.to_string()));
    let o: Option<Vec<u32>> = ['1', '2', 'z'].iter().map(|c| c.to_digit(10)).collect();
    println!("{:?}", o);
    let boxed: Box<dyn Error + Send + Sync> = From::from("plain message");
    println!("{} / {:?}", boxed, boxed.to_string());
}
