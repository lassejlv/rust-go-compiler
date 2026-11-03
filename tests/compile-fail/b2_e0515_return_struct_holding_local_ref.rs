//~ ERROR E0515
struct W<'a> { s: &'a str }
fn make<'a>() -> W<'a> {
    let s = String::from("x");
    W { s: &s }
}
fn main() { println!("{}", make().s); }
