//~ ERROR lifetime
fn to_static<'a>(x: &'a String) -> &'static str {
    x.as_str()
}
fn main() { let s = String::from("x"); println!("{}", to_static(&s)); }
