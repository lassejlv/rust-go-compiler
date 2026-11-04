//~ ERROR lifetime
fn seconds<'a>(v: &[(&'static str, &'a str)]) -> Vec<&'static str> {
    v.iter().enumerate().map(|(_, p)| p.1).collect()
}
fn main() {
    let s = String::from("v");
    println!("{:?}", seconds(&[("a", s.as_str())]));
}
