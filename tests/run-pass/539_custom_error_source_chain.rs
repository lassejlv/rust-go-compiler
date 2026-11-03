// custom Error types with source() chains, walking the chain, downcasting sources
use std::error::Error;
use std::fmt;

#[derive(Debug)]
struct IoLike {
    path: String,
}
impl fmt::Display for IoLike {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "cannot open {}", self.path)
    }
}
impl Error for IoLike {}

#[derive(Debug)]
enum ConfigError {
    Missing(IoLike),
    Parse { line: usize, source: std::num::ParseIntError },
    Invalid(String),
}
impl fmt::Display for ConfigError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            ConfigError::Missing(_) => write!(f, "config missing"),
            ConfigError::Parse { line, .. } => write!(f, "parse error on line {}", line),
            ConfigError::Invalid(s) => write!(f, "invalid config: {}", s),
        }
    }
}
impl Error for ConfigError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        match self {
            ConfigError::Missing(e) => Some(e),
            ConfigError::Parse { source, .. } => Some(source),
            ConfigError::Invalid(_) => None,
        }
    }
}

#[derive(Debug)]
struct AppError {
    ctx: &'static str,
    inner: ConfigError,
}
impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "app failed during {}", self.ctx)
    }
}
impl Error for AppError {
    fn source(&self) -> Option<&(dyn Error + 'static)> {
        Some(&self.inner)
    }
}

fn load(name: &str) -> Result<i32, ConfigError> {
    match name {
        "missing" => Err(ConfigError::Missing(IoLike { path: format!("/etc/{}.conf", name) })),
        "bad" => {
            let e = "12x".parse::<i32>().unwrap_err();
            Err(ConfigError::Parse { line: 3, source: e })
        }
        "neg" => Err(ConfigError::Invalid("negative".into())),
        _ => Ok(name.len() as i32),
    }
}

fn run(name: &str) -> Result<i32, AppError> {
    let v = load(name).map_err(|inner| AppError { ctx: "startup", inner })?;
    Ok(v * 2)
}

fn chain(e: &dyn Error) -> Vec<String> {
    let mut out = vec![e.to_string()];
    let mut cur = e.source();
    while let Some(s) = cur {
        out.push(s.to_string());
        cur = s.source();
    }
    out
}

fn root_cause<'a>(e: &'a (dyn Error + 'static)) -> &'a (dyn Error + 'static) {
    let mut cur = e;
    while let Some(s) = cur.source() {
        cur = s;
    }
    cur
}

fn main() {
    for n in ["ok", "missing", "bad", "neg"] {
        match run(n) {
            Ok(v) => println!("{} -> {}", n, v),
            Err(e) => {
                println!("{} -> {}", n, chain(&e).join(" <- "));
                let root = root_cause(&e);
                if let Some(io) = root.downcast_ref::<IoLike>() {
                    println!("  root is IoLike for {}", io.path);
                } else if let Some(pe) = root.downcast_ref::<std::num::ParseIntError>() {
                    println!("  root is ParseIntError: {:?}", pe.kind());
                } else if root.is::<ConfigError>() {
                    println!("  root is ConfigError");
                }
                let src = e.source().unwrap();
                println!("  direct source is ConfigError: {}", src.is::<ConfigError>());
                println!("  debug: {:?}", e.inner);
            }
        }
    }
    let boxed: Box<dyn Error> = Box::new(run("bad").unwrap_err());
    println!("{}", boxed);
    let mut depth = 0;
    let mut cur: Option<&dyn Error> = Some(boxed.as_ref());
    while let Some(c) = cur {
        depth += 1;
        cur = c.source();
    }
    println!("depth {}", depth);
    let as_app = boxed.downcast::<AppError>().unwrap();
    println!("ctx {}", as_app.ctx);
    let errs: Vec<Box<dyn Error>> = vec![
        Box::new(IoLike { path: "x".into() }),
        Box::new(ConfigError::Invalid("y".into())),
        "plain string error".into(),
        Box::new(fmt::Error),
    ];
    for e in &errs {
        println!("{} | source? {}", e, e.source().is_some());
    }
    let s: Box<dyn Error + Send + Sync> = From::from(String::from("owned msg"));
    println!("{:?} {}", s.to_string(), s);
}
