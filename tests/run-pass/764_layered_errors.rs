// Layered error handling across three "modules": low-level parse errors,
// mid-level config errors with source() chains, and an app-level error
// enum; From conversions for `?`, Box<dyn Error + Send + Sync>, downcasting
// through the chain, map_err/context-like helpers, and error reports.
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

mod lowlevel {
    use std::fmt;
    #[derive(Debug, PartialEq)]
    pub enum LexError { UnexpectedChar { ch: char, at: usize }, Unterminated(usize) }
    impl fmt::Display for LexError {
        fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
            match self {
                LexError::UnexpectedChar { ch, at } => write!(f, "unexpected {:?} at {}", ch, at),
                LexError::Unterminated(at) => write!(f, "unterminated string starting at {}", at),
            }
        }
    }
    impl std::error::Error for LexError {}
    pub fn check(line: &str) -> Result<(), LexError> {
        if let Some(i) = line.find('"') {
            if line[i + 1..].find('"').is_none() { return Err(LexError::Unterminated(i)); }
        }
        match line.char_indices().find(|(_, c)| *c == '@' || *c == '$') {
            Some((at, ch)) => Err(LexError::UnexpectedChar { ch, at }),
            None => Ok(()),
        }
    }
}

#[derive(Debug)]
enum ConfigError {
    Lex { line: usize, source: lowlevel::LexError },
    BadValue { key: String, source: ParseIntError },
    Missing(&'static str),
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Lex { line, .. } => write!(f, "syntax error on line {}", line),
            ConfigError::BadValue { key, .. } => write!(f, "invalid value for {:?}", key),
            ConfigError::Missing(k) => write!(f, "missing required key {:?}", k),
        }
    }
}
impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Lex { source, .. } => Some(source),
            ConfigError::BadValue { source, .. } => Some(source),
            ConfigError::Missing(_) => None,
        }
    }
}

#[derive(Debug)]
struct Config { port: u16, workers: u8, name: String }

fn parse_config(text: &str) -> Result<Config, ConfigError> {
    let (mut port, mut workers, mut name) = (None, None, None);
    for (i, line) in text.lines().enumerate() {
        lowlevel::check(line).map_err(|e| ConfigError::Lex { line: i + 1, source: e })?;
        let Some((k, v)) = line.split_once('=') else { continue };
        let (k, v) = (k.trim(), v.trim());
        let num = |v: &str| v.parse::<i64>().map_err(|e| ConfigError::BadValue { key: k.to_string(), source: e });
        match k {
            "port" => port = Some(num(v)? as u16),
            "workers" => workers = Some(num(v)? as u8),
            "name" => name = Some(v.trim_matches('"').to_string()),
            _ => {}
        }
    }
    Ok(Config { port: port.ok_or(ConfigError::Missing("port"))?, workers: workers.unwrap_or(1), name: name.ok_or(ConfigError::Missing("name"))? })
}

#[derive(Debug)]
enum AppError { Config(ConfigError), Validation(String), Other(Box<dyn Error + Send + Sync>) }
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self { AppError::Config(_) => write!(f, "failed to load configuration"), AppError::Validation(m) => write!(f, "validation: {}", m), AppError::Other(e) => write!(f, "other: {}", e) }
    }
}
impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self { AppError::Config(e) => Some(e), AppError::Other(e) => Some(e.as_ref()), _ => None }
    }
}
impl From<ConfigError> for AppError { fn from(e: ConfigError) -> Self { AppError::Config(e) } }
impl From<std::str::Utf8Error> for AppError { fn from(e: std::str::Utf8Error) -> Self { AppError::Other(Box::new(e)) } }

fn start(raw: &[u8]) -> Result<String, AppError> {
    let text = std::str::from_utf8(raw)?;
    let cfg = parse_config(text)?;
    if cfg.workers == 0 { return Err(AppError::Validation("workers must be > 0".into())); }
    if cfg.port < 1024 { return Err(AppError::Other(format!("port {} is privileged", cfg.port).into())); }
    Ok(format!("started {} on :{} with {} workers", cfg.name, cfg.port, cfg.workers))
}

fn report(e: &dyn Error) -> String {
    let mut s = e.to_string();
    let mut cur = e.source();
    let mut depth = 0;
    while let Some(c) = cur {
        depth += 1;
        s.push_str(&format!("\n    {}caused by: {}", "  ".repeat(depth - 1), c));
        cur = c.source();
    }
    s
}

fn root_cause<'a>(e: &'a (dyn Error + 'static)) -> &'a (dyn Error + 'static) { match e.source() { Some(s) => root_cause(s), None => e } }

fn main() {
    let inputs: Vec<&[u8]> = vec![
        b"name = \"svc\"\nport = 8080\nworkers = 4",
        b"name = \"svc\"\nport = 80x",
        b"name = \"svc\nport = 9000",
        b"port = 9000\nworkers = 2\nuser = a@b",
        b"port = 9000",
        b"name=x\nport=9000\nworkers=0",
        b"name=x\nport=22",
        b"name=\xff\xfe",
        b"name=big\nport=99999999999999999999",
    ];
    for (i, raw) in inputs.iter().enumerate() {
        match start(raw) {
            Ok(msg) => println!("[{}] ok: {}", i, msg),
            Err(e) => {
                println!("[{}] error: {}", i, report(&e));
                let root = root_cause(&e);
                let kind = if root.downcast_ref::<ParseIntError>().is_some() { "ParseIntError" }
                    else if root.downcast_ref::<lowlevel::LexError>().is_some() { "LexError" }
                    else if root.is::<std::str::Utf8Error>() { "Utf8Error" }
                    else if root.is::<AppError>() { "AppError" }
                    else if root.is::<ConfigError>() { "ConfigError" }
                    else { "opaque" };
                println!("     root kind: {} / debug: {:?}", kind, root.to_string());
            }
        }
    }
    let boxed: Box<dyn Error + Send + Sync> = Box::new(lowlevel::LexError::Unterminated(3));
    let s: Box<dyn Error> = boxed;
    println!("boxed downcast: {:?}", s.downcast::<lowlevel::LexError>().map(|b| *b));
    let from_str: Box<dyn Error> = "plain message".into();
    let from_string: Box<dyn Error + Send + Sync> = String::from("owned message").into();
    println!("{} | {} | {:?}", from_str, from_string, from_str.source().is_none());
}
