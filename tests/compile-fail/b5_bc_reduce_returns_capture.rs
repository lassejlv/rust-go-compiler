//~ ERROR lifetime
fn red<'a>(v: &[(&'static str, &'static str)], x: &'a str) -> Option<&'static str> {
    v.iter().map(|p| p.1).reduce(|_, _| x)
}
fn main() {
    let s = String::from("v");
    println!("{:?}", red(&[("a", "b"), ("c", "d")], &s));
}
