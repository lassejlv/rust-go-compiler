use std::panic::Location;
#[track_caller]
fn whereami() -> &'static Location<'static> { Location::caller() }
#[track_caller]
fn outer() -> u32 { whereami().line() }
fn main() {
    let l = whereami();
    println!("{}:{}", l.line(), l.column());
    println!("{}", outer());
    let v: Vec<i32> = vec![];
    let r = std::panic::catch_unwind(|| { let x: Option<i32> = None; x.unwrap() }); println!("{}", r.is_err());
    let _ = (r, v);
}
