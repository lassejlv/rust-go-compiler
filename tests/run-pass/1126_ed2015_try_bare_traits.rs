// edition:2015
// Edition 2015: try! macro, bare trait objects (Box<Trait>, &Trait),
// anonymous trait method parameters, and `dyn`/`async`/`await`/`try` as identifiers.
use std::error::Error;
use std::fmt;
use std::num::ParseIntError;

#[derive(Debug)]
enum AppError {
    Parse(ParseIntError),
    Negative(i64),
    TooBig { value: i64, max: i64 },
}

impl fmt::Display for AppError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        match *self {
            AppError::Parse(ref e) => write!(f, "parse error: {}", e),
            AppError::Negative(v) => write!(f, "negative value {}", v),
            AppError::TooBig { value, max } => write!(f, "{} exceeds {}", value, max),
        }
    }
}

impl Error for AppError {
    fn description(&self) -> &str {
        "app error"
    }
}

impl From<ParseIntError> for AppError {
    fn from(e: ParseIntError) -> AppError {
        AppError::Parse(e)
    }
}

fn parse_checked(s: &str, max: i64) -> Result<i64, AppError> {
    let v: i64 = try!(s.trim().parse());
    if v < 0 {
        return Err(AppError::Negative(v));
    }
    if v > max {
        return Err(AppError::TooBig { value: v, max: max });
    }
    Ok(v)
}

fn sum_all(items: &[&str]) -> Result<i64, Box<Error>> {
    let mut total = 0;
    for it in items {
        total += try!(parse_checked(it, 1000));
    }
    Ok(total)
}

trait Shape {
    fn area(&self) -> f64;
    fn scale(&mut self, f64);
    fn describe(&self, &str) -> String;
}

struct Circle(f64);
struct Rect(f64, f64);

impl Shape for Circle {
    fn area(&self) -> f64 { 3.0 * self.0 * self.0 }
    fn scale(&mut self, k: f64) { self.0 *= k; }
    fn describe(&self, prefix: &str) -> String { format!("{}circle r={}", prefix, self.0) }
}

impl Shape for Rect {
    fn area(&self) -> f64 { self.0 * self.1 }
    fn scale(&mut self, k: f64) { self.0 *= k; self.1 *= k; }
    fn describe(&self, prefix: &str) -> String { format!("{}rect {}x{}", prefix, self.0, self.1) }
}

fn total_area(shapes: &[Box<Shape>]) -> f64 {
    shapes.iter().map(|s| s.area()).sum()
}

fn apply(f: &Fn(i32) -> i32, x: i32) -> i32 {
    f(x)
}

fn make_adder(n: i32) -> Box<Fn(i32) -> i32> {
    Box::new(move |x| x + n)
}

fn largest<'a>(shapes: &'a [Box<Shape>]) -> &'a Shape {
    let mut best: &Shape = &*shapes[0];
    for s in shapes.iter() {
        if s.area() > best.area() {
            best = &**s;
        }
    }
    best
}

fn main() {
    for input in &["42", " 7 ", "-3", "5000", "abc", ""] {
        match parse_checked(input, 1000) {
            Ok(v) => println!("{:?} -> ok {}", input, v),
            Err(e) => println!("{:?} -> err {} ({})", input, e, e.description()),
        }
    }
    println!("{:?}", sum_all(&["1", "2", "3"]).ok());
    match sum_all(&["1", "x", "3"]) {
        Ok(_) => println!("unexpected"),
        Err(e) => println!("sum failed: {}", e),
    }

    let mut shapes: Vec<Box<Shape>> = vec![Box::new(Circle(1.0)), Box::new(Rect(2.0, 3.0)), Box::new(Circle(0.5))];
    println!("total area {}", total_area(&shapes));
    for s in shapes.iter_mut() {
        s.scale(2.0);
    }
    for (i, s) in shapes.iter().enumerate() {
        println!("{}", s.describe(&format!("#{} ", i)));
    }
    println!("largest: {}", largest(&shapes).describe(""));
    println!("total area {}", total_area(&shapes));

    let dyn = 10;
    let async = dyn * 2;
    let await = async + 1;
    let try = |x: i32| x * await;
    println!("dyn={} async={} await={} try(2)={}", dyn, async, await, try(2));
    let double = |x| x * 2;
    println!("apply: {} {}", apply(&double, 21), apply(&*make_adder(100), 5));
    let fs: Vec<Box<Fn(i32) -> i32>> = vec![make_adder(1), Box::new(|x| x * x), make_adder(-10)];
    println!("{:?}", fs.iter().map(|f| f(7)).collect::<Vec<_>>());
    let r: &Error = &AppError::Negative(-1);
    println!("as &Error: {}", r);
}
