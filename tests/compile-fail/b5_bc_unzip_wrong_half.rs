//~ ERROR lifetime
fn second<'a>(v: Vec<(&'static str, &'a str)>) -> Vec<&'static str> {
    let (_a, b): (Vec<&'static str>, Vec<_>) = v.into_iter().unzip();
    b
}
fn main() {
    let s = String::from("x");
    println!("{:?}", second(vec![("a", s.as_str())]));
}
