macro_rules! otry { ($e:expr) => { match $e { Some(e) => e, None => panic!("Failed") } }; }
fn ver() -> (u32, bool) {
    let s = "rustc 1.80.0";
    let mut pieces = s.split('.');
    let _ = pieces.next();
    let minor = pieces.next();
    let minor = otry!(otry!(minor).parse().ok());
    println!("detected rust 1.{minor}");
    (minor, false)
}
fn main() { println!("{:?}", ver()); }
