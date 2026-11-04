// ExitCode from main via Termination, with output flushed; std::env::args.
use std::process::ExitCode;

#[derive(Debug)]
enum AppError {
    BadInput(String),
}

fn run(args: &[String]) -> Result<u32, AppError> {
    let mut total = 0;
    for a in args {
        total += a.parse::<u32>().map_err(|_| AppError::BadInput(a.clone()))?;
    }
    Ok(total)
}

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    println!("args {:?}", args);
    match run(&args[..2]) {
        Ok(t) => println!("total {}", t),
        Err(e) => println!("err {:?}", e),
    }
    match run(&args) {
        Ok(t) => {
            println!("total {}", t);
            ExitCode::SUCCESS
        }
        Err(AppError::BadInput(s)) => {
            println!("bad input {}", s);
            ExitCode::from(42)
        }
    }
}
