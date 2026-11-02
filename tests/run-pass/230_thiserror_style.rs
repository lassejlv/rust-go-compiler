// thiserror-style error enums with manual Display, From conversions and source() chains.
use std::error::Error;
use std::fmt;
use std::num::{ParseFloatError, ParseIntError};

#[derive(Debug)]
enum ConfigError {
    Missing { key: String },
    BadInt { key: String, source: ParseIntError },
    BadFloat(ParseFloatError),
    Io(IoError),
    Nested(Box<ConfigError>),
}

#[derive(Debug)]
struct IoError {
    path: String,
    code: i32,
}

impl fmt::Display for IoError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "io error {} reading {}", self.code, self.path)
    }
}
impl Error for IoError {}

impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            ConfigError::Missing { key } => write!(f, "missing key `{}`", key),
            ConfigError::BadInt { key, .. } => write!(f, "invalid integer for `{}`", key),
            ConfigError::BadFloat(_) => f.write_str("invalid float"),
            ConfigError::Io(_) => f.write_str("i/o failure"),
            ConfigError::Nested(inner) => write!(f, "while loading include: {}", inner),
        }
    }
}

impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Missing { .. } => None,
            ConfigError::BadInt { source, .. } => Some(source),
            ConfigError::BadFloat(e) => Some(e),
            ConfigError::Io(e) => Some(e),
            ConfigError::Nested(e) => Some(e.as_ref()),
        }
    }
}

impl From<ParseFloatError> for ConfigError {
    fn from(e: ParseFloatError) -> Self {
        ConfigError::BadFloat(e)
    }
}
impl From<IoError> for ConfigError {
    fn from(e: IoError) -> Self {
        ConfigError::Io(e)
    }
}

struct Source<'a> {
    pairs: Vec<(&'a str, &'a str)>,
}

impl<'a> Source<'a> {
    fn parse(text: &'a str) -> Result<Self, IoError> {
        if text.starts_with("!io") {
            return Err(IoError { path: "config.ini".into(), code: 13 });
        }
        Ok(Source { pairs: text.lines().filter_map(|l| l.split_once('=')).map(|(k, v)| (k.trim(), v.trim())).collect() })
    }
    fn get(&self, key: &str) -> Result<&'a str, ConfigError> {
        self.pairs.iter().find(|(k, _)| *k == key).map(|(_, v)| *v).ok_or_else(|| ConfigError::Missing { key: key.to_string() })
    }
    fn get_int(&self, key: &str) -> Result<i64, ConfigError> {
        let v = self.get(key)?;
        v.parse().map_err(|source| ConfigError::BadInt { key: key.to_string(), source })
    }
}

#[derive(Debug)]
struct Config {
    threads: i64,
    scale: f64,
    name: String,
}

fn load(text: &str) -> Result<Config, ConfigError> {
    let src = Source::parse(text)?;
    let threads = src.get_int("threads")?;
    let scale: f64 = src.get("scale")?.parse()?;
    let name = src.get("name").unwrap_or("default").to_string();
    if let Ok(inc) = src.get("include") {
        load(&inc.replace(';', "\n")).map_err(|e| ConfigError::Nested(Box::new(e)))?;
    }
    Ok(Config { threads, scale, name })
}

fn report(e: &dyn Error) -> String {
    let mut s = format!("error: {}", e);
    let mut cur = e.source();
    let mut depth = 0;
    while let Some(c) = cur {
        depth += 1;
        s += &format!("\n  {}caused by: {}", "  ".repeat(depth - 1), c);
        cur = c.source();
    }
    s
}

fn main() {
    let inputs = [
        "threads = 4\nscale = 1.5\nname = fast",
        "scale = 2",
        "threads = four\nscale = 1",
        "threads = 2\nscale = x1.0",
        "!io",
        "threads = 1\nscale = 0.5\ninclude = threads=7;scale=oops",
        "threads = 1\nscale = 0.5\ninclude = threads=x",
    ];
    for inp in inputs {
        match load(inp) {
            Ok(c) => println!("ok: {:?}", c),
            Err(e) => {
                println!("{}", report(&e));
                let is_missing = matches!(e, ConfigError::Missing { .. });
                let has_src = e.source().is_some();
                println!("  [missing={} has_source={}]", is_missing, has_src);
            }
        }
    }
    let boxed: Box<dyn Error> = Box::new(ConfigError::Missing { key: "k".into() });
    println!("{} / {:?}", boxed, boxed);
    let as_err: Box<dyn Error + Send + Sync> = "plain string error".into();
    println!("{} source={:?}", as_err, as_err.source().is_none());
}
