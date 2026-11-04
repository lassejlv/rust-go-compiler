//~ ERROR E0597
fn ext(out: &mut Vec<&'static str>) {
    let s = String::from("a b");
    out.extend(s.split(' '));
}
fn main() {
    let mut out = Vec::new();
    ext(&mut out);
    println!("{:?}", out);
}
