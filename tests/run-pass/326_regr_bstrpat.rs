fn f(e: &str) -> Option<&'static str> { let s = match e.as_bytes() { b"lt" => "<", b"amp" => "&", _ => return None }; Some(s) }
fn main() { println!("{:?} {:?} {:?}", f("lt"), f("amp"), f("x")); }
