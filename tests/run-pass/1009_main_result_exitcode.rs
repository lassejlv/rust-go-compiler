use std::fmt;
use std::process::ExitCode;

#[derive(Debug)]
enum AppError { Parse(String), Range(i64) }

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match self {
            AppError::Parse(s) => write!(f, "cannot parse {:?}", s),
            AppError::Range(n) => write!(f, "{} out of range", n),
        }
    }
}

fn check(s: &str) -> Result<i64, AppError> {
    let n: i64 = s.trim().parse().map_err(|_| AppError::Parse(s.to_string()))?;
    if !(0..100).contains(&n) { return Err(AppError::Range(n)); }
    Ok(n)
}

fn main() -> ExitCode {
    let inputs = ["5", " 42 ", "abc", "500"];
    let mut failures = 0u8;
    for i in inputs {
        match check(i) {
            Ok(n) => println!("ok {}", n),
            Err(e) => { println!("error: {}", e); failures += 1; }
        }
    }
    println!("{:?}", ExitCode::SUCCESS == ExitCode::from(0));
    if failures > 0 {
        ExitCode::from(40 + failures)
    } else {
        ExitCode::SUCCESS
    }
}
