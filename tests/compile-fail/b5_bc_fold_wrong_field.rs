//~ ERROR lifetime
fn firsts<'a>(v: &[(&'a str, &'static str)]) -> Vec<&'static str> {
    v.iter().fold(Vec::new(), |mut acc, p| {
        acc.push(p.0);
        acc
    })
}
fn main() {
    let s = String::from("x");
    println!("{:?}", firsts(&[(s.as_str(), "b")]));
}
