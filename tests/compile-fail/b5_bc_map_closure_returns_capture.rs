//~ ERROR lifetime
fn all<'a>(v: &[(&'static str, &'a str)], x: &'a str) -> Vec<&'static str> {
    v.iter().map(|_| x).collect()
}
fn main() {
    let s = String::from("x");
    println!("{:?}", all(&[("a", "b")], &s));
}
