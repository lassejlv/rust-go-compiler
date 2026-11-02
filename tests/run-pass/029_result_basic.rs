#[derive(Debug)]
enum MathError {
    DivisionByZero,
    NegativeSqrt(f64),
    Overflow,
}

fn divide(a: i32, b: i32) -> Result<i32, MathError> {
    if b == 0 {
        Err(MathError::DivisionByZero)
    } else {
        Ok(a / b)
    }
}

fn sqrt(x: f64) -> Result<f64, MathError> {
    if x < 0.0 {
        Err(MathError::NegativeSqrt(x))
    } else {
        Ok(x.sqrt())
    }
}

fn mul(a: i32, b: i32) -> Result<i32, MathError> {
    a.checked_mul(b).ok_or(MathError::Overflow)
}

fn describe(r: &Result<i32, MathError>) -> String {
    match r {
        Ok(v) => format!("ok: {}", v),
        Err(MathError::DivisionByZero) => "error: division by zero".to_string(),
        Err(MathError::Overflow) => "error: overflow".to_string(),
        Err(e) => format!("error: {:?}", e),
    }
}

fn main() {
    println!("{}", describe(&divide(10, 2)));
    println!("{}", describe(&divide(1, 0)));
    println!("{}", describe(&mul(100000, 100000)));
    println!("{}", describe(&mul(1000, 1000)));
    println!("{:?}", sqrt(16.0));
    println!("{:?}", sqrt(-4.0));
    let r: Result<i32, String> = Ok(3);
    println!("is_ok = {}, is_err = {}", r.is_ok(), r.is_err());
    println!("map = {:?}", r.clone().map(|x| x * 2));
    let e: Result<i32, String> = Err("bad".to_string());
    println!("map_err = {:?}", e.clone().map_err(|s| s.len()));
    println!("unwrap_or = {}", e.clone().unwrap_or(0));
    println!("unwrap_or_else = {}", e.clone().unwrap_or_else(|s| s.len() as i32 * 100));
    println!("ok() = {:?}, err() = {:?}", r.clone().ok(), e.clone().err());
    println!("and_then = {:?}", r.clone().and_then(|x| if x > 2 { Ok(x * 10) } else { Err("small".to_string()) }));
    println!("unwrap = {}", r.clone().unwrap());
    println!("expect_err = {}", e.clone().expect_err("should be err"));
    let parsed: Vec<Result<i32, std::num::ParseIntError>> = vec!["1", "x", "3"].into_iter().map(|s| s.parse::<i32>()).collect();
    for p in &parsed {
        match p {
            Ok(n) => println!("parsed {}", n),
            Err(err) => println!("parse error: {}", err),
        }
    }
    let all: Result<Vec<i32>, _> = "4 5 6".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("all = {:?}", all);
    let bad: Result<Vec<i32>, _> = "4 five 6".split(' ').map(|s| s.parse::<i32>()).collect();
    println!("bad is_err = {}", bad.is_err());
    if let Ok(v) = divide(9, 3) {
        println!("if let ok: {}", v);
    }
    if let Err(MathError::DivisionByZero) = divide(9, 0) {
        println!("if let err: division by zero");
    }
}
