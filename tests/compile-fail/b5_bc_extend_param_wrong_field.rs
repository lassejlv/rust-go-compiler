//~ ERROR lifetime
fn ext<'a>(out: &mut Vec<&'static str>, v: &[(&'static str, &'a str)]) {
    out.extend(v.iter().map(|p| p.1));
}
fn main() {
    let s = String::from("x");
    let mut out = Vec::new();
    ext(&mut out, &[("a", s.as_str())]);
    println!("{:?}", out);
}
